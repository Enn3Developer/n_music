//! The play session as the queue page shows it, and what dragging an item in it does: a port
//! of the Qt app's `src/bridge/queue.rs`, without its animations.

use crate::rows::QueueRow;
use n_music_core::queue::ItemId;

/// An entry of the session as the queue page lists it.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct QueueLayoutRow {
    pub entry: QueueRow,
    /// Played before the current item: dimmed, and it can't be dragged.
    pub history: bool,
    /// The current item.
    pub current: bool,
}

/// The session split into what played, the current item and what is still to play.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct QueueLayout {
    /// What played, if asked for, then the current item, if asked for, then what is left.
    pub rows: Vec<QueueLayoutRow>,
    /// The items played before the current one, listed or not.
    pub history: u32,
    /// Queued items still to play: "Clear queued" shows while there are some.
    pub up_next: u32,
    /// Items still to play: "%1 left".
    pub left: u32,
    /// The row of the current item; `None` when it is not listed.
    pub current_row: Option<u32>,
}

/// What dropping a dragged item does.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct QueueMove {
    /// The item moved.
    pub item: ItemId,
    /// What it now plays before; `None` after everything.
    pub before: Option<ItemId>,
    /// It is the current item: send `Command.MoveCurrent`, else `Command.MoveUpcoming`.
    pub current: bool,
    /// The rows as they stand once it landed, until the session reports the move: the queued
    /// items the current one passed going down play right after it.
    pub rows: Vec<QueueLayoutRow>,
}

/// `entries`, the session in play order as `QueueChanged` reports it, split around `current`,
/// the current item. Played items are listed with `show_history`, the current one with
/// `show_current`.
#[uniffi::export]
pub fn queue_layout(
    entries: Vec<QueueRow>,
    current: Option<ItemId>,
    show_history: bool,
    show_current: bool,
) -> QueueLayout {
    let position =
        current.and_then(|current| entries.iter().position(|entry| entry.item == current));
    let (mut history, mut up_next, mut left) = (0u32, 0u32, 0u32);
    let mut rows = Vec::with_capacity(entries.len());
    for (index, entry) in entries.into_iter().enumerate() {
        let current = position == Some(index);
        let played = position.is_some_and(|position| index < position);
        if current && !show_current {
            continue;
        }
        if played {
            history += 1;
            if !show_history {
                continue;
            }
        } else if !current {
            left += 1;
            up_next += u32::from(entry.queued);
        }
        rows.push(QueueLayoutRow {
            entry,
            history: played,
            current,
        });
    }
    let current_row = rows
        .iter()
        .position(|row| row.current)
        .map(|row| row as u32);
    QueueLayout {
        rows,
        history,
        up_next,
        left,
        current_row,
    }
}

/// Drops the item at `from` of `rows`, as `queue_layout` listed them, at `to`. Only the
/// current item and those still to play move; one still to play goes among those played too,
/// above the current item, and then counts as played and is no longer queued. The current
/// item moved down makes the context's items it passes played, while the queued ones it passes
/// play right after it. `None` when nothing moves.
#[uniffi::export]
pub fn queue_move(rows: Vec<QueueLayoutRow>, from: u32, to: u32) -> Option<QueueMove> {
    let mut rows = rows;
    let from = from as usize;
    let dragged = rows.get(from).filter(|row| !row.history)?;
    let (item, current, queued) = (dragged.entry.item, dragged.current, dragged.entry.queued);
    // The rows played before the current item, the first ones.
    let played = rows.iter().take_while(|row| row.history).count();
    // Those still to play go among those played too, above the current item.
    let first = if current || rows.iter().any(|row| row.current) {
        0
    } else {
        rows.iter()
            .position(|row| !row.history && !row.current)
            .unwrap_or(from)
    };
    let to = (to as usize).min(rows.len() - 1).max(first);
    if to == from {
        return None;
    }
    let row = rows.remove(from);
    rows.insert(to, row);
    if current {
        let first = from.min(to);
        for (index, row) in rows
            .iter_mut()
            .enumerate()
            .take(from.max(to) + 1)
            .skip(first)
        {
            row.history = index < to && (index < played || !row.entry.queued);
        }
    } else {
        let history = rows
            .iter()
            .position(|row| row.current)
            .is_some_and(|current| to < current);
        rows[to].history = history;
        rows[to].entry.queued = queued && !history;
    }
    let before = rows.get(to + 1).map(|row| row.entry.item);
    if current && to > from {
        // The queued items it passed going down play right after it.
        let mut kept = Vec::with_capacity(rows.len());
        let mut passed = Vec::new();
        for (index, row) in rows.into_iter().enumerate() {
            if (from..to).contains(&index) && row.entry.queued {
                passed.push(row);
            } else {
                kept.push(row);
            }
        }
        let place = kept
            .iter()
            .position(|row| row.entry.item == item)
            .map_or(kept.len(), |place| place + 1);
        kept.splice(place..place, passed);
        rows = kept;
    }
    Some(QueueMove {
        item,
        before,
        current,
        rows,
    })
}
