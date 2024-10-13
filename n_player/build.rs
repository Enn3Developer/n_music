use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    println!("cargo::rerun-if-changed=ui/");
    slint_build::compile("ui/window.slint").expect("Slint build failed");
    let lang_dir = Path::new("assets").join("lang").read_dir().unwrap();
    let mut localizations = String::from("const LOCALIZATIONS: [(&str, &str); {LEN}] = [");
    let mut get_locale = String::from("pub fn get_locale(denominator: &str) -> Locale { serde_json::from_str(match denominator { ");
    let mut len = 0;
    for locale_file in lang_dir {
        if let Ok(locale_file) = locale_file {
            let file_name = locale_file.file_name();
            let name = file_name.to_str().unwrap();
            let localization = name.rsplit('.').last().unwrap();
            let mut split = localization.split('_');
            let denominator = split.next().unwrap();
            let locale_name = split.next().unwrap();
            localizations.push_str(&format!("(\"{denominator}\", \"{locale_name}\"),"));
            len += 1;
            if denominator != "en" {
                get_locale.push_str(&format!("\"{denominator}\" => include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/assets/lang/{name}\")),"));
            }
        }
    }
    localizations.push_str("];");
    localizations = localizations.replace("{LEN}", &len.to_string());
    get_locale.push_str("_ => include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/assets/lang/en_English.json\"))}).unwrap()}");
    let mut content = localizations;
    content.push('\n');
    content.push_str(&get_locale);
    let mut localization_file =
        File::create(Path::new(&std::env::var_os("OUT_DIR").unwrap()).join("localizations.rs"))
            .unwrap();
    localization_file.write_all(content.as_bytes()).unwrap();
}
