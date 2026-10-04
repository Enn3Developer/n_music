//! Storage Access Framework access: lists and opens documents of a picked folder through
//! Android's `ContentResolver`. Only framework classes are used, so the calls work from any
//! attached native thread (app classes would not be found by their class loader).

use jni::objects::{Global, JObject, JString};
use jni::{jni_sig, jni_str, Env, JavaVM};
use n_music_core::source::{version_stamp, Locator, OpenedStream, StreamProvider, TrackEntry};
use std::fs::File;
use std::io;
use std::os::fd::{FromRawFd, OwnedFd};
use std::sync::Arc;

const DIRECTORY_MIME: &str = "vnd.android.document/directory";
/// Android derives MIME types from extensions and does not know every audio container
/// (or reports Ogg as `application/ogg`), so these are accepted regardless of MIME type.
const AUDIO_EXTENSIONS: &[&str] = &[
    "aac", "aif", "aiff", "caf", "dca", "flac", "m4a", "mka", "mp3", "oga", "ogg", "opus", "wav",
    "webm",
];

/// `DocumentsContract.Document` columns read for each child, in cursor order.
const COLUMNS: [&str; 5] = [
    "document_id",
    "_display_name",
    "mime_type",
    "_size",
    "last_modified",
];

struct Child {
    uri: String,
    name: String,
    version: Option<u64>,
}

pub struct AndroidDocumentProvider {
    jvm: Arc<JavaVM>,
    resolver: Global<JObject<'static>>,
}

impl AndroidDocumentProvider {
    pub fn new(jvm: Arc<JavaVM>, context: &Global<JObject<'static>>) -> jni::errors::Result<Self> {
        let resolver = jvm.attach_current_thread(|env| -> jni::errors::Result<_> {
            let resolver = env
                .call_method(
                    context.as_ref(),
                    jni_str!("getContentResolver"),
                    jni_sig!("()Landroid/content/ContentResolver;"),
                    &[],
                )?
                .l()?;
            env.new_global_ref(&resolver)
        })?;
        Ok(Self { jvm, resolver })
    }

    fn open_fd(&self, uri: &str) -> jni::errors::Result<i32> {
        self.jvm.attach_current_thread(|env| {
            env.with_local_frame(4, |env| -> jni::errors::Result<i32> {
                let uri = parse_uri(env, uri)?;
                let mode = env.new_string("r")?;
                let descriptor = env
                    .call_method(
                        self.resolver.as_ref(),
                        jni_str!("openFileDescriptor"),
                        jni_sig!(
                            "(Landroid/net/Uri;Ljava/lang/String;)Landroid/os/ParcelFileDescriptor;"
                        ),
                        &[(&uri).into(), (&mode).into()],
                    )?
                    .l()?;
                if descriptor.is_null() {
                    return Err(jni::errors::Error::NullPtr("openFileDescriptor"));
                }
                env.call_method(&descriptor, jni_str!("detachFd"), jni_sig!("()I"), &[])?
                    .i()
            })
        })
    }

    /// Returns the audio files directly inside `tree`.
    fn list_children(&self, tree: &str) -> jni::errors::Result<Vec<Child>> {
        self.jvm.attach_current_thread(|env| {
            env.with_local_frame(16, |env| -> jni::errors::Result<_> {
                let tree = parse_uri(env, tree)?;
                let tree_id = documents_call(
                    env,
                    jni_str!("getTreeDocumentId"),
                    &jni_sig!("(Landroid/net/Uri;)Ljava/lang/String;"),
                    &[(&tree).into()],
                )?;
                let children = documents_call(
                    env,
                    jni_str!("buildChildDocumentsUriUsingTree"),
                    &jni_sig!("(Landroid/net/Uri;Ljava/lang/String;)Landroid/net/Uri;"),
                    &[(&tree).into(), (&tree_id).into()],
                )?;
                let projection = env.new_object_array(
                    COLUMNS.len() as i32,
                    jni_str!("java/lang/String"),
                    JObject::null(),
                )?;
                for (index, column) in COLUMNS.iter().enumerate() {
                    let column = env.new_string(column)?;
                    projection.set_element(env, index, &column)?;
                }
                let cursor = env
                    .call_method(
                        self.resolver.as_ref(),
                        jni_str!("query"),
                        jni_sig!("(Landroid/net/Uri;[Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;Ljava/lang/String;)Landroid/database/Cursor;"),
                        &[
                            (&children).into(),
                            (&projection).into(),
                            (&JObject::null()).into(),
                            (&JObject::null()).into(),
                            (&JObject::null()).into(),
                        ],
                    )?
                    .l()?;
                if cursor.is_null() {
                    return Err(jni::errors::Error::NullPtr("ContentResolver.query"));
                }
                let rows = read_rows(env, &cursor, &tree);
                close_cursor(env, &cursor)?;
                rows
            })
        })
    }
}

impl StreamProvider for AndroidDocumentProvider {
    fn open(&self, locator: &Locator) -> io::Result<OpenedStream> {
        let Locator::Document { uri, .. } = locator else {
            return Err(unsupported(locator));
        };
        let fd = self
            .open_fd(uri)
            .map_err(|error| io::Error::other(format!("Could not open {uri}: {error}")))?;
        if fd < 0 {
            return Err(io::Error::other(format!(
                "Could not open {uri}: invalid descriptor"
            )));
        }
        // SAFETY: `detachFd` transfers ownership of the descriptor to us, nothing else closes it.
        let file = File::from(unsafe { OwnedFd::from_raw_fd(fd) });
        Ok(OpenedStream {
            source: Box::new(file),
            extension: locator.extension(),
        })
    }

    fn list_tracks(&self, root: &Locator) -> io::Result<Vec<TrackEntry>> {
        let Locator::DocumentTree(tree) = root else {
            return Err(unsupported(root));
        };
        let children = self
            .list_children(tree)
            .map_err(|error| io::Error::other(format!("Could not list {tree}: {error}")))?;
        Ok(children
            .into_iter()
            .map(|child| TrackEntry {
                locator: Locator::Document {
                    uri: child.uri,
                    name: child.name,
                },
                version: child.version,
            })
            .collect())
    }
}

fn read_rows(env: &mut Env, cursor: &JObject, tree: &JObject) -> jni::errors::Result<Vec<Child>> {
    let mut rows = vec![];
    while env
        .call_method(cursor, jni_str!("moveToNext"), jni_sig!("()Z"), &[])?
        .z()?
    {
        // A frame per row keeps local references bounded on large folders.
        let row = env.with_local_frame(8, |env| -> jni::errors::Result<_> {
            let id = cursor_string(env, cursor, 0)?;
            let name = cursor_string(env, cursor, 1)?;
            let mime = cursor_string(env, cursor, 2)?;
            let (Some(id), Some(name)) = (id, name) else {
                return Ok(None);
            };
            let mime = mime.unwrap_or_default();
            if mime == DIRECTORY_MIME || !is_audio(&name, &mime) {
                return Ok(None);
            }
            let size = cursor_long(env, cursor, 3)?;
            let modified = cursor_long(env, cursor, 4)?;
            let version = size
                .zip(modified)
                .map(|(size, modified)| version_stamp(size as u64, modified as u64));
            let id = env.new_string(id)?;
            let uri = documents_call(
                env,
                jni_str!("buildDocumentUriUsingTree"),
                &jni_sig!("(Landroid/net/Uri;Ljava/lang/String;)Landroid/net/Uri;"),
                &[tree.into(), (&id).into()],
            )?;
            let uri = env
                .call_method(
                    &uri,
                    jni_str!("toString"),
                    jni_sig!("()Ljava/lang/String;"),
                    &[],
                )?
                .l()?;
            Ok(Some(Child {
                uri: to_string(env, uri)?,
                name,
                version,
            }))
        })?;
        rows.extend(row);
    }
    Ok(rows)
}

/// Closes the cursor even when reading it threw, then re-raises that exception.
fn close_cursor(env: &mut Env, cursor: &JObject) -> jni::errors::Result<()> {
    let pending = env.exception_occurred();
    env.exception_clear();
    let closed = env.call_method(cursor, jni_str!("close"), jni_sig!("()V"), &[]);
    if let Some(exception) = pending {
        env.exception_clear();
        env.throw(exception)?;
    }
    closed.map(drop)
}

fn is_audio(name: &str, mime: &str) -> bool {
    mime.starts_with("audio/")
        || std::path::Path::new(name)
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
}

fn cursor_string<'local>(
    env: &mut Env<'local>,
    cursor: &JObject,
    column: i32,
) -> jni::errors::Result<Option<String>> {
    let value = env
        .call_method(
            cursor,
            jni_str!("getString"),
            jni_sig!("(I)Ljava/lang/String;"),
            &[column.into()],
        )?
        .l()?;
    if value.is_null() {
        return Ok(None);
    }
    to_string(env, value).map(Some)
}

fn cursor_long(env: &mut Env, cursor: &JObject, column: i32) -> jni::errors::Result<Option<i64>> {
    let null = env
        .call_method(
            cursor,
            jni_str!("isNull"),
            jni_sig!("(I)Z"),
            &[column.into()],
        )?
        .z()?;
    if null {
        return Ok(None);
    }
    env.call_method(
        cursor,
        jni_str!("getLong"),
        jni_sig!("(I)J"),
        &[column.into()],
    )?
    .j()
    .map(Some)
}

fn to_string<'local>(env: &mut Env<'local>, value: JObject<'local>) -> jni::errors::Result<String> {
    JString::cast_local(env, value)?.try_to_string(env)
}

fn parse_uri<'local>(env: &mut Env<'local>, uri: &str) -> jni::errors::Result<JObject<'local>> {
    let uri = env.new_string(uri)?;
    env.call_static_method(
        jni_str!("android/net/Uri"),
        jni_str!("parse"),
        jni_sig!("(Ljava/lang/String;)Landroid/net/Uri;"),
        &[(&uri).into()],
    )?
    .l()
}

fn documents_call<'local>(
    env: &mut Env<'local>,
    name: &jni::strings::JNIStr,
    sig: &jni::signature::MethodSignature,
    args: &[jni::objects::JValue],
) -> jni::errors::Result<JObject<'local>> {
    env.call_static_method(
        jni_str!("android/provider/DocumentsContract"),
        name,
        sig,
        args,
    )?
    .l()
}

fn unsupported(locator: &Locator) -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        format!("{locator} is not an Android document"),
    )
}

/// Human-readable form of a library root for the settings screen: a document tree is shown as
/// its decoded tree id (e.g. `primary:Music`).
pub fn display_path(path: &str) -> String {
    let Some((_, tree_id)) = path.split_once("/tree/") else {
        return path.to_string();
    };
    let tree_id = tree_id.split('/').next().unwrap_or(tree_id);
    let bytes = tree_id.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                decoded.push(byte);
                i += 3;
            }
            (byte, _) => {
                decoded.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}
