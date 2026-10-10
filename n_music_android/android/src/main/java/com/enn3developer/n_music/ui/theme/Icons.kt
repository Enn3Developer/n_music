package com.enn3developer.n_music.ui.theme

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.addPathNodes
import androidx.compose.ui.unit.dp

/**
 * The design's icons, from the desktop set: a 24 dp grid, drawn with a 1.9 dp stroke and round
 * caps, except the playback controls and ⋮, which are filled. Tint them with `Icon`.
 */
object NIcons {
    val Search by stroke("M4.5 11a6.5 6.5 0 1 0 13 0a6.5 6.5 0 1 0 -13 0M20 20l-4.2-4.2")
    val Settings by stroke(
        "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2zM15 12a3 3 0 1 1-6 0a3 3 0 1 1 6 0z"
    )
    val Library by stroke(
        "M8 3.5h10.5A1.5 1.5 0 0 1 20 5v10.5a1.5 1.5 0 0 1-1.5 1.5H8a1.5 1.5 0 0 1-1.5-1.5V5A1.5 1.5 0 0 1 8 3.5zM3.5 7.5V18A2.5 2.5 0 0 0 6 20.5h10.5M15 13V7.5h2.5M15 13a1.75 1.75 0 1 1-3.5 0a1.75 1.75 0 1 1 3.5 0z"
    )
    val Playlist by stroke("M4 6h11M4 11h11M4 16h6M18 8v9M14 17a2 2 0 1 0 4 0a2 2 0 1 0 -4 0")
    val Sources by stroke("M3.5 7a2 2 0 0 1 2-2h4l2 2.5h7a2 2 0 0 1 2 2V17a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z")
    val Web by stroke(
        "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"
    )
    val Telegram by stroke("M20.5 3.5L3.5 10.5l6.5 2.5 2.5 7zM20.5 3.5L10 13")
    val Group by stroke(
        "M5.6 8.5a3.2 3.2 0 1 0 6.4 0a3.2 3.2 0 1 0 -6.4 0M2.5 19.5c.7-3.3 3.2-5 6.3-5s5.6 1.7 6.3 5M15.3 5.5a3.2 3.2 0 0 1 0 6M17.7 14.8c1.9.6 3.2 2.2 3.8 4.7"
    )
    val Saved by stroke("M7 3.5h10a1 1 0 0 1 1 1v16l-6-4.2-6 4.2v-16a1 1 0 0 1 1-1z")
    val Later by stroke("M7 18h10.5a4 4 0 0 0 .6-8A6 6 0 0 0 6.6 9.2 4.5 4.5 0 0 0 7 18z")
    val Album by stroke("M3.5 12a8.5 8.5 0 1 0 17 0a8.5 8.5 0 1 0 -17 0M9.5 12a2.5 2.5 0 1 0 5 0a2.5 2.5 0 1 0 -5 0")
    val Artist by stroke("M8.5 8.5a3.5 3.5 0 1 0 7 0a3.5 3.5 0 1 0 -7 0M5 20c.8-3.6 3.6-5.5 7-5.5s6.2 1.9 7 5.5")
    val Genre by stroke("M3.5 12.5V4.5h8l9 9-8 8zM6.7 9a1.3 1.3 0 1 0 2.6 0a1.3 1.3 0 1 0 -2.6 0")
    val Filter by stroke("M4 5h16l-6 7.5V19l-4-2v-4.5z")
    val Sort by stroke("M7 4v16M4 17l3 3 3-3M14 6h7M14 11h5M14 16h3")
    val Reverse by stroke("M8 4v16M4 8l4-4 4 4M16 20V4M12 16l4 4 4-4")
    val Shuffle by stroke(
        "M3 7h3c2.2 0 3.4 1.2 4.5 3l3 4.5c1.1 1.8 2.3 3 4.5 3h3M18 14.5l3 3-3 3M3 17.5h3c1.4 0 2.4-.5 3.2-1.4M14.8 8.4c.8-.9 1.8-1.4 3.2-1.4h3M18 4l3 3-3 3"
    )
    val Repeat by stroke("M4 11V9a3 3 0 0 1 3-3h12M16 3l3 3-3 3M20 13v2a3 3 0 0 1-3 3H5M8 21l-3-3 3-3")
    val RepeatOne by stroke(
        "M4 11V9a3 3 0 0 1 3-3h12M16 3l3 3-3 3M20 13v2a3 3 0 0 1-3 3H5M8 21l-3-3 3-3M11 10.5l1.5-1v5"
    )
    val PlayNext by stroke("M4 6h12M4 11h12M4 16h7M15.5 15.2v5.6l4.6-2.8z")
    val AddToQueue by stroke("M4 6h12M4 11h12M4 16h7M17.5 14v6M14.5 17h6")
    val Drag by stroke("M5 9h14M5 15h14")
    val History by stroke("M3.5 12a8.5 8.5 0 1 0 2.5-6M3.5 4v4h4M12 7.5V12l3 2")
    val Output by stroke(
        "M7.5 3h9a1.5 1.5 0 0 1 1.5 1.5v15a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 6 19.5v-15A1.5 1.5 0 0 1 7.5 3zM11 18h2"
    )
    val Add by stroke("M12 5v14M5 12h14")
    val Close by stroke("M6 6l12 12M18 6L6 18")
    val Check by stroke("M5 12.5l4.5 4.5L19 7.5")
    val Back by stroke("M19 12H5M11 6l-6 6 6 6")
    val Collapse by stroke("M6 9l6 6 6-6")
    val Expand by stroke("M6 15l6-6 6 6")
    val Open by stroke("M9 6l6 6-6 6")
    val Rename by stroke("M4 20h4L19 9l-4-4L4 16z")
    val Remove by stroke("M5 7h14M10 7V4.5h4V7M7 7l1 13h8l1-13")
    val Update by stroke(
        "M20 11a8 8 0 0 0-14.5-4.5L4 8M4 4v4h4M4 13a8 8 0 0 0 14.5 4.5L20 16M20 20v-4h-4"
    )
    val Alert by stroke("M3 12a9 9 0 1 0 18 0a9 9 0 1 0 -18 0M12 7.5v5.5M12 16.5v.01")
    val Info by stroke("M3.5 12a8.5 8.5 0 1 0 17 0a8.5 8.5 0 1 0 -17 0M12 11v5.5M12 7.8v.01")
    val Levelling by stroke("M5 18V10M9.5 18V6M14 18v-6M18.5 18V8")
    val Palette by stroke(
        "M12 3.5a8.5 8.5 0 0 0 0 17c1 0 1.6-.7 1.6-1.5 0-.9-.8-1.3-.8-2.2 0-.9.7-1.6 1.6-1.6h2a4.1 4.1 0 0 0 4.1-4.1C20.5 6.8 16.7 3.5 12 3.5zM7.5 12.5v.01M9.5 8v.01M14.5 8v.01M17 11.5v.01"
    )
    val Language by stroke(
        "M3.5 12a8.5 8.5 0 1 0 17 0a8.5 8.5 0 1 0 -17 0M3.5 12h17M12 3.5c2.3 2.4 3.5 5.2 3.5 8.5s-1.2 6.1-3.5 8.5c-2.3-2.4-3.5-5.2-3.5-8.5s1.2-6.1 3.5-8.5z"
    )
    val Share by stroke(
        "M15.5 5a2.5 2.5 0 1 0 5 0a2.5 2.5 0 1 0 -5 0M3.5 12a2.5 2.5 0 1 0 5 0a2.5 2.5 0 1 0 -5 0M15.5 19a2.5 2.5 0 1 0 5 0a2.5 2.5 0 1 0 -5 0M15.6 6.3l-7.2 4.4M8.4 13.3l7.2 4.4"
    )
    val SleepTimer by stroke("M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z")
    val SelectAll by stroke("M3.5 7l2.5 2.5L10 5.5M3.5 14l2.5 2.5L10 12.5M13.5 8h7M13.5 15h7M3.5 20h17")
    val Locked by stroke(
        "M6.5 10.5h11a1.5 1.5 0 0 1 1.5 1.5v7a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 5 19v-7a1.5 1.5 0 0 1 1.5-1.5zM8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5"
    )
    val RemoveSource by stroke("M3.5 12a8.5 8.5 0 1 0 17 0a8.5 8.5 0 1 0 -17 0M8 12h8")
    val Tag by stroke("M3.5 12.5V4.5h8l9 9-8 8zM6.7 9a1.3 1.3 0 1 0 2.6 0a1.3 1.3 0 1 0 -2.6 0")
    val MiniPlayer by stroke(
        "M5 7h14a2.5 2.5 0 0 1 2.5 2.5v5a2.5 2.5 0 0 1-2.5 2.5H5a2.5 2.5 0 0 1-2.5-2.5v-5A2.5 2.5 0 0 1 5 7zM6.5 10.5h2v3h-2zM14 10.2v3.6l3-1.8z"
    )
    val GridView by stroke(
        "M5.5 4h3A1.5 1.5 0 0 1 10 5.5v3A1.5 1.5 0 0 1 8.5 10h-3A1.5 1.5 0 0 1 4 8.5v-3A1.5 1.5 0 0 1 5.5 4zM15.5 4h3A1.5 1.5 0 0 1 20 5.5v3A1.5 1.5 0 0 1 18.5 10h-3A1.5 1.5 0 0 1 14 8.5v-3A1.5 1.5 0 0 1 15.5 4zM5.5 14h3A1.5 1.5 0 0 1 10 15.5v3A1.5 1.5 0 0 1 8.5 20h-3A1.5 1.5 0 0 1 4 18.5v-3A1.5 1.5 0 0 1 5.5 14zM15.5 14h3A1.5 1.5 0 0 1 20 15.5v3A1.5 1.5 0 0 1 18.5 20h-3A1.5 1.5 0 0 1 14 18.5v-3A1.5 1.5 0 0 1 15.5 14z"
    )
    val ListView by stroke(
        "M5 4.5h2a1 1 0 0 1 1 1v2a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-2a1 1 0 0 1 1-1zM11.5 6.5H20M5 10h2a1 1 0 0 1 1 1v2a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-2a1 1 0 0 1 1-1zM11.5 12H20M5 15.5h2a1 1 0 0 1 1 1v2a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-2a1 1 0 0 1 1-1zM11.5 17.5H20"
    )
    val Menu by stroke("M4 7h16M4 12h16M4 17h16")
    val MenuOpen by stroke("M4 7h16M4 12h10M4 17h16")
    val Logs by stroke(
        "M6.5 3.5h7l4 4v11.5a1.5 1.5 0 0 1-1.5 1.5h-9.5A1.5 1.5 0 0 1 5 19V5a1.5 1.5 0 0 1 1.5-1.5zM13.5 3.5v4h4M8.5 12.5h7M8.5 16h5"
    )
    val Code by stroke("M8.5 7L3.5 12l5 5M15.5 7l5 5-5 5")
    val External by stroke(
        "M14 4h6v6M20 4l-8.5 8.5M18 14v4.5a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 4 18.5v-11A1.5 1.5 0 0 1 5.5 6H10"
    )

    val Play by fill("M7 5.14v13.72a1 1 0 0 0 1.5.86l11-6.86a1 1 0 0 0 0-1.72l-11-6.86A1 1 0 0 0 7 5.14z")
    val Pause by fill(
        "M6 5.5A1.5 1.5 0 0 1 7.5 4h2A1.5 1.5 0 0 1 11 5.5v13A1.5 1.5 0 0 1 9.5 20h-2A1.5 1.5 0 0 1 6 18.5zM13 5.5A1.5 1.5 0 0 1 14.5 4h2A1.5 1.5 0 0 1 18 5.5v13a1.5 1.5 0 0 1-1.5 1.5h-2a1.5 1.5 0 0 1-1.5-1.5z"
    )
    val Previous by fill(
        "M19 6.1v11.8a1 1 0 0 1-1.55.83l-8.4-5.9a1 1 0 0 1 0-1.66l8.4-5.9A1 1 0 0 1 19 6.1zM7 5.5a1 1 0 0 0-1-1h-.5a1 1 0 0 0-1 1v13a1 1 0 0 0 1 1H6a1 1 0 0 0 1-1z"
    )
    val Next by fill(
        "M5 6.1v11.8a1 1 0 0 0 1.55.83l8.4-5.9a1 1 0 0 0 0-1.66l-8.4-5.9A1 1 0 0 0 5 6.1zM17 5.5a1 1 0 0 1 1-1h.5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1H18a1 1 0 0 1-1-1z"
    )
    val More by fill(
        "M10.4 5.5a1.6 1.6 0 1 0 3.2 0a1.6 1.6 0 1 0 -3.2 0M10.4 12a1.6 1.6 0 1 0 3.2 0a1.6 1.6 0 1 0 -3.2 0M10.4 18.5a1.6 1.6 0 1 0 3.2 0a1.6 1.6 0 1 0 -3.2 0"
    )

    /** The check of a picked row or swatch, drawn bolder than the rest. */
    val CheckBold by stroke("M5 12.5l4.5 4.5L19 7.5", width = 2.6f)

    /** The filter icon as the big tile of a smart playlist draws it, at 44 dp. */
    val FilterThin by stroke("M4 5h16l-6 7.5V19l-4-2v-4.5z", width = 1.6f)

    /** The playlist icon as the big tile of an empty playlist draws it, at 44 dp. */
    val PlaylistThin by stroke("M4 6h11M4 11h11M4 16h6M18 8v9M14 17a2 2 0 1 0 4 0a2 2 0 1 0 -4 0", width = 1.6f)

    /** The alert of a source that can't be reached, at 14 dp. */
    val AlertBold by stroke("M3 12a9 9 0 1 0 18 0a9 9 0 1 0 -18 0M12 7.5v5.5M12 16.5v.01", width = 2.2f)
}

/** A stroked icon on the 24 dp grid. */
private fun stroke(path: String, width: Float = 1.9f) = lazy {
    ImageVector.Builder(
        defaultWidth = 24.dp,
        defaultHeight = 24.dp,
        viewportWidth = 24f,
        viewportHeight = 24f,
    ).addPath(
        pathData = addPathNodes(path),
        fill = null,
        stroke = SolidColor(Color.Black),
        strokeLineWidth = width,
        strokeLineCap = StrokeCap.Round,
        strokeLineJoin = StrokeJoin.Round,
    ).build()
}

/** A filled icon on the 24 dp grid. */
private fun fill(path: String) = lazy {
    ImageVector.Builder(
        defaultWidth = 24.dp,
        defaultHeight = 24.dp,
        viewportWidth = 24f,
        viewportHeight = 24f,
    ).addPath(pathData = addPathNodes(path), fill = SolidColor(Color.Black)).build()
}
