//! What plays and in which order: a snapshot of the context's tracks, the order they play in,
//! and the queued tracks, each played once where it stands among them.

use super::{ItemId, LoopStatus, QueueEntry};
use crate::library::catalog::Catalog;
use crate::library::query::Query;
use crate::library::user_data::{SessionItems, SessionState, StoredCursor, StoredItem};
use crate::source::Locator;
use crate::Track;
use rand::prelude::SliceRandom;
use rand::rng;

#[derive(Clone, Debug)]
pub(super) struct Item {
    pub id: ItemId,
    pub locator: Locator,
    pub fingerprint: Option<u64>,
}

/// A queued item and where it stands in the play order.
#[derive(Clone, Debug)]
struct Queued {
    item: Item,
    /// The slot of the context item it plays before; the order's length for after the last.
    before: usize,
}

/// One of the items still to play.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Upcoming {
    /// The context item at this position.
    Context(usize),
    /// `queued` at this index.
    Queued(usize),
}

/// Which item is current.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Current {
    /// The context item at the play order's cursor.
    Context,
    /// [`Session::detour`], a queued item.
    Detour,
}

#[derive(Default)]
pub(super) struct Session {
    context: Option<Query>,
    /// The context's tracks, in its order.
    items: Vec<Item>,
    order: PlayOrder,
    /// The cursor's context item has played: the context goes on after it, not from the start.
    started: bool,
    /// By where they stand, never before the slots still to play: those of the same slot in the
    /// order they play.
    queued: Vec<Queued>,
    /// The queued item playing now, taken off `queued`.
    detour: Option<Item>,
    current: Option<Current>,
    next_id: u64,
    /// The entries changed since they were last taken.
    changed: bool,
}

impl Session {
    pub fn context(&self) -> Option<&Query> {
        self.context.as_ref()
    }

    fn item(&mut self, locator: Locator, fingerprint: Option<u64>) -> Item {
        self.next_id += 1;
        Item {
            id: ItemId(self.next_id),
            locator,
            fingerprint,
        }
    }

    /// Replaces the context with `tracks`; the queued items stay, to play first. Returns what
    /// to play first: `start` when it is one of the tracks.
    pub fn replace_context(
        &mut self,
        query: Query,
        tracks: &[Track],
        start: Option<&Locator>,
        shuffle: bool,
    ) -> Option<ItemId> {
        self.items = tracks
            .iter()
            .map(|track| self.item(track.locator.clone(), track.fingerprint))
            .collect();
        let start = start.and_then(|start| tracks.iter().position(|t| &t.locator == start));
        self.order = PlayOrder::new(self.items.len(), shuffle, start);
        self.context = Some(query);
        self.started = false;
        for queued in &mut self.queued {
            queued.before = 0;
        }
        self.detour = None;
        self.current = None;
        self.changed = true;
        let first = start.or_else(|| self.order.first())?;
        self.items.get(first).map(|item| item.id)
    }

    /// The context item playing `locator`.
    pub fn find(&self, locator: &Locator) -> Option<ItemId> {
        self.items
            .iter()
            .find(|item| &item.locator == locator)
            .map(|item| item.id)
    }

    pub fn current(&self) -> Option<&Item> {
        match self.current? {
            Current::Context => self.items.get(self.order.current()?),
            Current::Detour => self.detour.as_ref(),
        }
    }

    pub fn get(&self, id: ItemId) -> Option<&Item> {
        self.items
            .iter()
            .chain(self.queued.iter().map(|queued| &queued.item))
            .chain(&self.detour)
            .find(|item| item.id == id)
    }

    /// The context item the context goes on after, if it started.
    fn context_position(&self) -> Option<usize> {
        self.started.then(|| self.order.current()).flatten()
    }

    /// The first slot still to play.
    fn split(&self) -> usize {
        self.context_position()
            .map_or(0, |position| self.order.slot_of[position] + 1)
    }

    /// The items still to play, in the order they play.
    fn upcoming(&self) -> Vec<Upcoming> {
        let len = self.order.order.len();
        let mut upcoming = Vec::with_capacity(len + self.queued.len());
        let mut queued = self.queued.iter().enumerate().peekable();
        for slot in self.split()..=len {
            while let Some((index, _)) = queued.next_if(|(_, queued)| queued.before <= slot) {
                upcoming.push(Upcoming::Queued(index));
            }
            if let Some(&position) = self.order.order.get(slot) {
                upcoming.push(Upcoming::Context(position));
            }
        }
        upcoming
    }

    fn upcoming_id(&self, upcoming: Upcoming) -> ItemId {
        match upcoming {
            Upcoming::Context(position) => self.items[position].id,
            Upcoming::Queued(index) => self.queued[index].item.id,
        }
    }

    /// Plays the items still to play in the order of `upcoming`, which lists each of them once.
    fn set_upcoming(&mut self, upcoming: &[Upcoming]) {
        let mut slot = self.split();
        let mut old: Vec<Option<Queued>> = std::mem::take(&mut self.queued)
            .into_iter()
            .map(Some)
            .collect();
        for &entry in upcoming {
            match entry {
                Upcoming::Context(position) => {
                    self.order.order[slot] = position;
                    slot += 1;
                }
                Upcoming::Queued(index) => {
                    if let Some(mut queued) = old[index].take() {
                        queued.before = slot;
                        self.queued.push(queued);
                    }
                }
            }
        }
        self.order.reindex();
        self.changed = true;
    }

    /// Moves `item`, one still to play, to play right before `before`, another one still to
    /// play, or (`None`) after all of them; the others keep their order. `before` playing now or
    /// played, it goes right before it among those played, counting as played: a queued item
    /// joins the context there.
    pub fn move_upcoming(&mut self, item: ItemId, before: Option<ItemId>) {
        let mut upcoming = self.upcoming();
        let Some(from) = upcoming
            .iter()
            .position(|&entry| self.upcoming_id(entry) == item)
        else {
            return;
        };
        let moved = upcoming.remove(from);
        let to = match before {
            None => upcoming.len(),
            Some(before) => match upcoming
                .iter()
                .position(|&entry| self.upcoming_id(entry) == before)
            {
                Some(to) => to,
                None => {
                    if let Some(slot) = self.played_slot(before) {
                        self.move_to_played(moved, slot);
                    }
                    return;
                }
            },
        };
        if to == from {
            return;
        }
        upcoming.insert(to, moved);
        self.set_upcoming(&upcoming);
    }

    /// The slot an item moved right before `before`, the current item or one that played, takes
    /// among those played: `before`'s own, or after them all for a queued item playing now.
    fn played_slot(&self, before: ItemId) -> Option<usize> {
        let split = self.split();
        if self.current == Some(Current::Detour)
            && self
                .detour
                .as_ref()
                .is_some_and(|detour| detour.id == before)
        {
            return Some(split);
        }
        let position = self.items.iter().position(|item| item.id == before)?;
        let slot = self.order.slot_of[position];
        (slot < split).then_some(slot)
    }

    /// Moves `moved`, an item still to play, to `slot`, at most the first slot still to play: it
    /// counts as played, and the current item stays current. A queued item joins the context,
    /// before the item it now plays before in the context's own order too.
    fn move_to_played(&mut self, moved: Upcoming, slot: usize) {
        match moved {
            Upcoming::Context(position) => {
                let from = self.order.slot_of[position];
                // Those from `slot` to its own move one slot on: the queued items keep their
                // places among the items still to play.
                for queued in &mut self.queued {
                    if queued.before <= from {
                        queued.before += 1;
                    }
                }
                self.order.order.remove(from);
                self.order.order.insert(slot, position);
            }
            Upcoming::Queued(index) => {
                let item = self.queued.remove(index).item;
                let position = self
                    .order
                    .order
                    .get(slot)
                    .copied()
                    .unwrap_or(self.items.len());
                self.items.insert(position, item);
                for other in &mut self.order.order {
                    if *other >= position {
                        *other += 1;
                    }
                }
                self.order.order.insert(slot, position);
                // The slots still to play all move one slot on.
                for queued in &mut self.queued {
                    queued.before += 1;
                }
                // The rounds made before go without it.
                self.clear_rounds();
            }
        }
        self.order.reindex();
        // The cursor follows its item, or starts the context with this one.
        if self.started {
            self.order.cursor += 1;
        } else {
            self.order.cursor = slot;
            self.started = true;
        }
        self.changed = true;
    }

    /// Moves the current item to play right before `before`, an item still to play or one that
    /// played, or (`None`) after all of them; it plays on, and once it ends playback goes on
    /// from there. The context items it passes going down count as played, while queued ones
    /// play right after it; those it passes going up play again after it. Returns whether it
    /// moved.
    pub fn move_current(&mut self, before: Option<ItemId>) -> bool {
        // A current context item leaves its slot to take another; the others shift.
        let own = match self.current {
            Some(Current::Context) if self.started => self.order.current(),
            Some(Current::Detour) => None,
            _ => return false,
        };
        let shift = usize::from(own.is_some());
        // It plays after the first `at` context slots, counted without its own.
        let at = self.split() - shift;
        let slot = |slot: usize| slot - usize::from(own.is_some() && slot > at);
        let to = match before {
            None => self.order.order.len() - shift,
            Some(before) => {
                if let Some(position) = self.items.iter().position(|item| item.id == before) {
                    slot(self.order.slot_of[position])
                } else if let Some(queued) = self.queued.iter().find(|q| q.item.id == before) {
                    slot(queued.before)
                } else {
                    return false;
                }
            }
        };
        if to == at {
            return false;
        }
        if let Some(position) = own {
            self.order.order.remove(at);
            self.order.order.insert(to, position);
            self.order.reindex();
            self.order.cursor = to;
        } else if to > 0 {
            self.order.cursor = to - 1;
            self.started = true;
        } else {
            self.started = false;
        }
        let split = self.split();
        for queued in &mut self.queued {
            queued.before = queued.before.max(split);
        }
        self.changed = true;
        true
    }

    /// Keeps each queued item behind as many context items still to play as before `change`,
    /// which reorders the context or moves where it stands.
    fn keeping_queued(&mut self, change: impl FnOnce(&mut Self)) {
        let split = self.split();
        let behind: Vec<usize> = self
            .queued
            .iter()
            .map(|queued| queued.before.saturating_sub(split))
            .collect();
        change(self);
        let (split, len) = (self.split(), self.order.order.len());
        for (queued, behind) in self.queued.iter_mut().zip(behind) {
            queued.before = (split + behind).min(len);
        }
    }

    /// What plays after the current item. `manual` is a skip: it leaves a repeated track and
    /// goes on past the end of the context even with looping off. `None` stops playback.
    pub fn next(
        &mut self,
        loop_status: &LoopStatus,
        shuffle: bool,
        manual: bool,
    ) -> Option<ItemId> {
        self.order.crossing = None;
        if !manual && *loop_status == LoopStatus::File {
            if let Some(current) = self.current() {
                return Some(current.id);
            }
        }
        let split = self.split();
        if let Some(queued) = self.queued.first().filter(|queued| queued.before <= split) {
            return Some(queued.item.id);
        }
        let position = match self.context_position() {
            None => self.order.first()?,
            Some(position) if !self.order.is_last(position) => self.order.after(position, false),
            Some(_) if !manual && *loop_status == LoopStatus::Off => return None,
            Some(position) => self.order.after(position, shuffle),
        };
        self.items.get(position).map(|item| item.id)
    }

    /// What plays on "previous": the item before the current one; at the very start, the
    /// current item again (the last one when looping the context).
    pub fn previous(&mut self, loop_status: &LoopStatus) -> Option<ItemId> {
        self.order.crossing = None;
        let position = match (self.current, self.context_position()) {
            // Queued items play once: back from one is back in the context.
            (Some(Current::Detour), Some(position)) => Some(position),
            (Some(Current::Detour), None) => return self.detour.as_ref().map(|item| item.id),
            (_, Some(position)) => self.order.before(position).or_else(|| {
                if *loop_status == LoopStatus::Playlist {
                    self.order.last()
                } else {
                    Some(position)
                }
            }),
            (_, None) => self.order.last(),
        }?;
        self.items.get(position).map(|item| item.id)
    }

    /// Makes `id` the current item: a queued item is taken off the list, a context item moves
    /// the cursor (into the next or previous round across a round boundary). The queued items
    /// it skips play right after it.
    pub fn arrive(&mut self, id: ItemId) {
        if self.current().is_some_and(|current| current.id == id) {
            return;
        }
        if let Some(index) = self.queued.iter().position(|queued| queued.item.id == id) {
            self.detour = Some(self.queued.remove(index).item);
            self.current = Some(Current::Detour);
            self.changed = true;
        } else if let Some(position) = self.items.iter().position(|item| item.id == id) {
            self.order.arrive(position);
            self.changed |= std::mem::take(&mut self.order.changed) || self.detour.is_some();
            self.detour = None;
            self.started = true;
            self.current = Some(Current::Context);
            let split = self.split();
            for queued in &mut self.queued {
                if queued.before < split {
                    queued.before = split;
                    self.changed = true;
                }
            }
        }
    }

    /// Forgets where `next` or `previous` was about to go: the user picked an item instead,
    /// which stays in the current round.
    pub fn forget_crossing(&mut self) {
        self.order.crossing = None;
    }

    /// Starts the context over after it ran out; a shuffled one in a new order.
    pub fn restart(&mut self, shuffle: bool) -> Option<ItemId> {
        self.keeping_queued(|session| {
            if shuffle {
                session.order = PlayOrder::new(session.items.len(), true, None);
                session.changed = true;
            }
            session.started = false;
        });
        self.next(&LoopStatus::Off, shuffle, true)
    }

    /// Queues tracks to play before the next context item, after what is queued there; `next`
    /// plays them first.
    pub fn enqueue(
        &mut self,
        tracks: impl IntoIterator<Item = (Locator, Option<u64>)>,
        next: bool,
    ) {
        let split = self.split();
        let items: Vec<Queued> = tracks
            .into_iter()
            .map(|(locator, fingerprint)| Queued {
                item: self.item(locator, fingerprint),
                before: split,
            })
            .collect();
        if items.is_empty() {
            return;
        }
        let at = if next {
            0
        } else {
            self.queued
                .iter()
                .take_while(|queued| queued.before <= split)
                .count()
        };
        self.queued.splice(at..at, items);
        self.changed = true;
    }

    pub fn remove_queued(&mut self, id: ItemId) {
        let before = self.queued.len();
        self.queued.retain(|queued| queued.item.id != id);
        self.changed |= self.queued.len() != before;
    }

    pub fn clear_queued(&mut self) {
        self.changed |= !self.queued.is_empty();
        self.queued.clear();
    }

    /// Shuffles the context from the current track on, or goes back to its own order.
    pub fn set_shuffle(&mut self, shuffle: bool) {
        self.keeping_queued(|session| {
            let current = session.order.current().filter(|_| session.started);
            session.order = PlayOrder::new(session.items.len(), shuffle, current);
        });
        self.changed = true;
    }

    /// Forgets the prepared and the past round, which belong to the old loop mode.
    pub fn clear_rounds(&mut self) {
        self.order.next_round = None;
        self.order.previous_round = None;
    }

    /// Drops the tracks the library no longer has, and follows those that moved. The current
    /// item stays even when gone: it may still be playing.
    pub fn reconcile(&mut self, catalog: &Catalog) {
        let current = self.current().map(|item| item.id);
        let mut changed = false;
        let mut keep = |item: &mut Item| {
            if catalog.track(&item.locator).is_some() {
                return true;
            }
            let moved = item
                .fingerprint
                .and_then(|fingerprint| catalog.track_by_fingerprint(fingerprint));
            match moved {
                Some(track) => {
                    item.locator = track.locator.clone();
                    changed = true;
                    true
                }
                None if Some(item.id) == current => true,
                None => {
                    changed = true;
                    false
                }
            }
        };
        let kept: Vec<bool> = self.items.iter_mut().map(&mut keep).collect();
        self.queued.retain_mut(|queued| keep(&mut queued.item));
        if let Some(detour) = &mut self.detour {
            keep(detour);
        }
        if !changed {
            return;
        }
        self.changed = true;
        if kept.iter().all(|&kept| kept) {
            return;
        }
        // The slots each queued item stood behind lose those dropped.
        let mut slots_kept = Vec::with_capacity(self.order.order.len() + 1);
        let mut count = 0;
        for &position in &self.order.order {
            slots_kept.push(count);
            count += usize::from(kept[position]);
        }
        slots_kept.push(count);
        for queued in &mut self.queued {
            queued.before = slots_kept[queued.before.min(slots_kept.len() - 1)];
        }
        let mut position = 0;
        let new_position: Vec<Option<usize>> = kept
            .iter()
            .map(|&kept| {
                kept.then(|| {
                    position += 1;
                    position - 1
                })
            })
            .collect();
        let mut kept = kept.iter();
        self.items.retain(|_| *kept.next().unwrap());
        self.order = self.order.remap(&new_position);
    }

    /// The entries, if they changed since last taken.
    pub fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    /// How many [`Session::entries`] there are.
    pub fn len(&self) -> usize {
        self.items.len() + usize::from(self.detour.is_some()) + self.queued.len()
    }

    /// Everything in play order: the context up to the current item, the queued item playing,
    /// then the items still to play.
    pub fn entries(&self) -> Vec<QueueEntry> {
        let entry = |item: &Item, queued: bool| QueueEntry {
            item: item.id,
            locator: item.locator.clone(),
            queued,
        };
        let mut entries: Vec<QueueEntry> = self.order.order[..self.split()]
            .iter()
            .map(|&position| entry(&self.items[position], false))
            .collect();
        entries.extend(self.detour.iter().map(|item| entry(item, true)));
        entries.extend(self.upcoming().into_iter().map(|upcoming| match upcoming {
            Upcoming::Context(position) => entry(&self.items[position], false),
            Upcoming::Queued(index) => entry(&self.queued[index].item, true),
        }));
        entries
    }

    pub fn stored_items(&self) -> SessionItems {
        let stored = |item: &Item| StoredItem {
            locator: item.locator.clone(),
            fingerprint: item.fingerprint,
        };
        SessionItems {
            context: self.context.clone(),
            items: self.items.iter().map(stored).collect(),
            slots: self.order.slot_of.clone(),
            up_next: self
                .queued
                .iter()
                .map(|queued| stored(&queued.item))
                .collect(),
            up_next_slots: self
                .queued
                .iter()
                .map(|queued| Some(queued.before))
                .collect(),
            detour: self.detour.as_ref().map(stored),
        }
    }

    pub fn stored_state(&self, position: f64, finished: bool) -> SessionState {
        SessionState {
            cursor: match self.current {
                Some(Current::Context) => self.order.current().map(StoredCursor::Context),
                Some(Current::Detour) => Some(StoredCursor::Detour(self.context_position())),
                None => None,
            },
            position,
            finished,
        }
    }

    /// A saved session; `None` when it does not fit together.
    pub fn restore(items: SessionItems, state: &SessionState) -> Option<Self> {
        let mut session = Session::default();
        let len = items.items.len();
        let mut order = vec![usize::MAX; len];
        for (position, &slot) in items.slots.iter().enumerate() {
            *order.get_mut(slot)? = position;
        }
        if order.contains(&usize::MAX) {
            return None;
        }
        let mut make = |stored: StoredItem| session.item(stored.locator, stored.fingerprint);
        let context: Vec<Item> = items.items.into_iter().map(&mut make).collect();
        let up_next: Vec<Item> = items.up_next.into_iter().map(&mut make).collect();
        let detour = items.detour.map(&mut make);
        session.items = context;
        session.context = items.context;
        let (current, started) = match state.cursor {
            Some(StoredCursor::Context(position)) if position < len => {
                (Some(Current::Context), Some(position))
            }
            Some(StoredCursor::Detour(after)) if detour.is_some() => {
                (Some(Current::Detour), after.filter(|&after| after < len))
            }
            _ => (None, None),
        };
        let cursor = started.map_or(0, |position| items.slots[position]);
        session.started = started.is_some();
        session.detour = detour;
        session.current = current;
        session.order = PlayOrder::from_order(order, cursor);
        // Saved without their places, they play before the context goes on.
        let split = session.split();
        let mut slots = items.up_next_slots.into_iter();
        session.queued = up_next
            .into_iter()
            .map(|item| Queued {
                item,
                before: slots.next().flatten().unwrap_or(split).clamp(split, len),
            })
            .collect();
        session.queued.sort_by_key(|queued| queued.before);
        session.changed = true;
        Some(session)
    }
}

/// The order context items play in, as positions in the context: its own order or a shuffle.
#[derive(Default)]
struct PlayOrder {
    order: Vec<usize>,
    /// Where each position is in `order`.
    slot_of: Vec<usize>,
    /// Slot of the current item.
    cursor: usize,
    /// The next shuffled round, made once the last slot needs a successor.
    next_round: Option<Vec<usize>>,
    /// The round before this one, for stepping back across the boundary.
    previous_round: Option<Vec<usize>>,
    /// The round boundary the last `after` or `before` crossed. A track can be both a round
    /// neighbour and an in-round one, so arriving at it says nothing about the direction.
    crossing: Option<Crossing>,
    /// `order` changed since this was last cleared.
    changed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Crossing {
    IntoNext,
    IntoPrevious,
}

impl PlayOrder {
    /// A fresh order of `len` items. When shuffling, `current` comes first and the rest
    /// follows in random order.
    fn new(len: usize, shuffle: bool, current: Option<usize>) -> Self {
        let mut order: Vec<usize> = (0..len).collect();
        let mut cursor = 0;
        if shuffle {
            order.shuffle(&mut rng());
            if let Some(current) = current {
                let slot = order
                    .iter()
                    .position(|&position| position == current)
                    .unwrap_or(0);
                order.swap(0, slot);
            }
        } else if let Some(current) = current {
            cursor = current;
        }
        Self::from_order(order, cursor)
    }

    fn from_order(order: Vec<usize>, cursor: usize) -> Self {
        let mut play_order = Self {
            order,
            slot_of: vec![],
            cursor,
            next_round: None,
            previous_round: None,
            crossing: None,
            changed: true,
        };
        play_order.reindex();
        play_order
    }

    /// Finds each position's slot again after `order` changed.
    fn reindex(&mut self) {
        self.slot_of = vec![0; self.order.len()];
        for (slot, &position) in self.order.iter().enumerate() {
            self.slot_of[position] = slot;
        }
    }

    fn current(&self) -> Option<usize> {
        self.order.get(self.cursor).copied()
    }

    fn first(&self) -> Option<usize> {
        self.order.first().copied()
    }

    fn last(&self) -> Option<usize> {
        self.order.last().copied()
    }

    fn is_last(&self, position: usize) -> bool {
        self.slot_of[position] + 1 == self.order.len()
    }

    /// What plays after `position`. Past the end the order wraps around; with `reshuffle` it
    /// starts a new round instead, never with the item that just played.
    fn after(&mut self, position: usize, reshuffle: bool) -> usize {
        let slot = self.slot_of[position];
        let len = self.order.len();
        if slot + 1 < len {
            return self.order[slot + 1];
        }
        if !reshuffle {
            return self.order[0];
        }
        self.crossing = Some(Crossing::IntoNext);
        self.next_round.get_or_insert_with(|| {
            let mut round: Vec<usize> = (0..len).collect();
            round.shuffle(&mut rng());
            if len > 1 && round[0] == position {
                round.swap(0, len - 1);
            }
            round
        })[0]
    }

    /// What played before `position`; from a round's first item, the previous round's last.
    /// `None` at the very start.
    fn before(&mut self, position: usize) -> Option<usize> {
        match self.slot_of[position] {
            0 => {
                let last = self.previous_round.as_ref()?.last().copied();
                self.crossing = Some(Crossing::IntoPrevious);
                last
            }
            slot => Some(self.order[slot - 1]),
        }
    }

    /// Moves to `position` without reordering anything, except across the round boundary the
    /// last `after` or `before` crossed: into the next round, or back into the previous one.
    fn arrive(&mut self, position: usize) {
        let last = self.order.len().saturating_sub(1);
        let crossing = self.crossing.take();
        if crossing == Some(Crossing::IntoNext)
            && self
                .next_round
                .as_ref()
                .is_some_and(|round| round[0] == position)
        {
            if let Some(round) = self.next_round.take() {
                let previous = std::mem::take(&mut self.order);
                *self = Self::from_order(round, 0);
                self.previous_round = Some(previous);
            }
        } else if crossing == Some(Crossing::IntoPrevious)
            && self
                .previous_round
                .as_ref()
                .is_some_and(|round| round.last() == Some(&position))
        {
            if let Some(round) = self.previous_round.take() {
                let next = std::mem::take(&mut self.order);
                *self = Self::from_order(round, last);
                self.next_round = Some(next);
            }
        } else {
            self.cursor = self.slot_of[position];
        }
    }

    /// The same order over fewer positions: `new_position` maps each old position to its new
    /// one, `None` for those dropped. The cursor's item must be kept.
    fn remap(&self, new_position: &[Option<usize>]) -> Self {
        let current = self.current().and_then(|position| new_position[position]);
        let order: Vec<usize> = self
            .order
            .iter()
            .filter_map(|&position| new_position[position])
            .collect();
        let mut remapped = Self::from_order(order, 0);
        if let Some(current) = current {
            remapped.cursor = remapped.slot_of[current];
        }
        remapped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TrackInfo;
    use std::sync::Arc;

    /// A session over the tracks named `0` to `len - 1`, in their order, not started.
    fn session(len: usize) -> Session {
        let tracks: Vec<Track> = (0..len)
            .map(|index| Arc::new(TrackInfo::placeholder(Locator::Local(index.to_string()))))
            .collect();
        let mut session = Session::default();
        session.replace_context(Query::library(), &tracks, None, false);
        session
    }

    fn id(session: &Session, name: &str) -> ItemId {
        session
            .entries()
            .into_iter()
            .find(|entry| entry.locator == Locator::Local(name.to_string()))
            .map(|entry| entry.item)
            .unwrap()
    }

    fn enqueue(session: &mut Session, names: &[&str]) {
        let tracks = names
            .iter()
            .map(|name| (Locator::Local(name.to_string()), None));
        session.enqueue(tracks, false);
    }

    /// The entries by name in play order, the current one in brackets.
    fn layout(session: &Session) -> String {
        let current = session.current().map(|item| item.id);
        session
            .entries()
            .into_iter()
            .map(|entry| {
                let Locator::Local(name) = entry.locator else {
                    unreachable!()
                };
                if Some(entry.item) == current {
                    format!("[{name}]")
                } else {
                    name
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn next(session: &mut Session) -> String {
        let next = session.next(&LoopStatus::Off, false, true).unwrap();
        let entries = session.entries();
        let entry = entries.iter().find(|entry| entry.item == next).unwrap();
        let Locator::Local(name) = &entry.locator else {
            unreachable!()
        };
        name.clone()
    }

    #[test]
    fn moves_upcoming_among_those_still_to_play() {
        let mut session = session(6);
        session.arrive(id(&session, "2"));
        session.move_upcoming(id(&session, "5"), Some(id(&session, "3")));
        assert_eq!(layout(&session), "0 1 [2] 5 3 4");
        session.move_upcoming(id(&session, "5"), None);
        assert_eq!(layout(&session), "0 1 [2] 3 4 5");
    }

    #[test]
    fn moves_upcoming_context_item_among_those_played() {
        let mut session = session(6);
        session.arrive(id(&session, "2"));
        session.move_upcoming(id(&session, "4"), Some(id(&session, "1")));
        assert_eq!(layout(&session), "0 4 1 [2] 3 5");
        assert_eq!(next(&mut session), "3");

        // Right before the current item, it played last.
        session.move_upcoming(id(&session, "3"), Some(id(&session, "2")));
        assert_eq!(layout(&session), "0 4 1 3 [2] 5");
        assert_eq!(next(&mut session), "5");
        let previous = session.previous(&LoopStatus::Off);
        assert_eq!(previous, Some(id(&session, "3")));
    }

    #[test]
    fn keeps_queued_items_in_place_when_moving_among_those_played() {
        let mut session = session(6);
        session.arrive(id(&session, "2"));
        enqueue(&mut session, &["a", "b"]);
        session.move_upcoming(id(&session, "b"), Some(id(&session, "4")));
        assert_eq!(layout(&session), "0 1 [2] a 3 b 4 5");
        session.move_upcoming(id(&session, "4"), Some(id(&session, "0")));
        assert_eq!(layout(&session), "4 0 1 [2] a 3 b 5");
        session.move_upcoming(id(&session, "3"), Some(id(&session, "2")));
        assert_eq!(layout(&session), "4 0 1 3 [2] a b 5");
    }

    #[test]
    fn moves_queued_item_into_the_context_among_those_played() {
        let mut session = session(4);
        session.arrive(id(&session, "1"));
        enqueue(&mut session, &["a", "b"]);
        session.move_upcoming(id(&session, "b"), Some(id(&session, "1")));
        assert_eq!(layout(&session), "0 b [1] a 2 3");
        let queued: Vec<bool> = session.entries().iter().map(|entry| entry.queued).collect();
        assert_eq!(queued, [false, false, false, true, false, false]);
        assert_eq!(next(&mut session), "a");
        // It keeps its place in the context's own order.
        session.set_shuffle(false);
        assert_eq!(layout(&session), "0 b [1] a 2 3");

        // Right before a queued item playing now, it played last.
        session.arrive(id(&session, "a"));
        enqueue(&mut session, &["c"]);
        assert_eq!(layout(&session), "0 b 1 [a] c 2 3");
        session.move_upcoming(id(&session, "c"), Some(id(&session, "a")));
        assert_eq!(layout(&session), "0 b 1 c [a] 2 3");
        assert_eq!(next(&mut session), "2");
    }

    #[test]
    fn moves_upcoming_context_item_among_those_played_before_a_queued_item() {
        let mut session = session(5);
        session.arrive(id(&session, "1"));
        enqueue(&mut session, &["a"]);
        session.arrive(id(&session, "a"));
        assert_eq!(layout(&session), "0 1 [a] 2 3 4");
        session.move_upcoming(id(&session, "3"), Some(id(&session, "a")));
        assert_eq!(layout(&session), "0 1 3 [a] 2 4");
        assert_eq!(next(&mut session), "2");
        session.move_upcoming(id(&session, "4"), Some(id(&session, "1")));
        assert_eq!(layout(&session), "0 4 1 3 [a] 2");
    }

    #[test]
    fn starts_the_context_with_an_item_moved_before_the_first_queued_one() {
        let mut session = session(4);
        enqueue(&mut session, &["a"]);
        session.arrive(id(&session, "a"));
        assert_eq!(layout(&session), "[a] 0 1 2 3");
        session.move_upcoming(id(&session, "2"), Some(id(&session, "a")));
        assert_eq!(layout(&session), "2 [a] 0 1 3");
        assert_eq!(next(&mut session), "0");
    }
}
