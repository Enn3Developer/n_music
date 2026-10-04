//! What plays and in which order: a snapshot of the context's tracks, the order they play in,
//! and the up-next tracks played once before the context goes on.

use super::{ItemId, LoopStatus, QueueEntry};
use crate::library::catalog::Catalog;
use crate::library::query::Query;
use crate::source::Locator;
use crate::Track;
use rand::prelude::SliceRandom;
use rand::rng;
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub(super) struct Item {
    pub id: ItemId,
    pub locator: Locator,
    pub fingerprint: Option<u64>,
}

/// Which item is current.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Current {
    /// The context item at the play order's cursor.
    Context,
    /// [`Session::detour`], an up-next item.
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
    up_next: VecDeque<Item>,
    /// The up-next item playing now, taken off `up_next`.
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

    /// Replaces the context with `tracks`; up next stays. Returns what to play first: `start`
    /// when it is one of the tracks.
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
            .chain(&self.up_next)
            .chain(&self.detour)
            .find(|item| item.id == id)
    }

    /// The context item the context goes on after, if it started.
    fn context_position(&self) -> Option<usize> {
        self.started.then(|| self.order.current()).flatten()
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
        if let Some(queued) = self.up_next.front() {
            return Some(queued.id);
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
            // Up-next items play once: back from one is back in the context.
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

    /// Makes `id` the current item: an up-next item is taken off the list, a context item
    /// moves the cursor (into the next or previous round across a round boundary).
    pub fn arrive(&mut self, id: ItemId) {
        if self.current().is_some_and(|current| current.id == id) {
            return;
        }
        if let Some(index) = self.up_next.iter().position(|item| item.id == id) {
            self.detour = self.up_next.remove(index);
            self.current = Some(Current::Detour);
            self.changed = true;
        } else if let Some(position) = self.items.iter().position(|item| item.id == id) {
            self.order.arrive(position);
            self.changed |= std::mem::take(&mut self.order.changed) || self.detour.is_some();
            self.detour = None;
            self.started = true;
            self.current = Some(Current::Context);
        }
    }

    /// Forgets where `next` or `previous` was about to go: the user picked an item instead,
    /// which stays in the current round.
    pub fn forget_crossing(&mut self) {
        self.order.crossing = None;
    }

    /// Starts the context over after it ran out; a shuffled one in a new order.
    pub fn restart(&mut self, shuffle: bool) -> Option<ItemId> {
        if shuffle {
            self.order = PlayOrder::new(self.items.len(), true, None);
            self.changed = true;
        }
        self.started = false;
        self.next(&LoopStatus::Off, shuffle, true)
    }

    pub fn enqueue(
        &mut self,
        tracks: impl IntoIterator<Item = (Locator, Option<u64>)>,
        next: bool,
    ) {
        let items: Vec<Item> = tracks
            .into_iter()
            .map(|(locator, fingerprint)| self.item(locator, fingerprint))
            .collect();
        if items.is_empty() {
            return;
        }
        if next {
            for item in items.into_iter().rev() {
                self.up_next.push_front(item);
            }
        } else {
            self.up_next.extend(items);
        }
        self.changed = true;
    }

    pub fn remove_queued(&mut self, id: ItemId) {
        let before = self.up_next.len();
        self.up_next.retain(|item| item.id != id);
        self.changed |= self.up_next.len() != before;
    }

    pub fn clear_queued(&mut self) {
        self.changed |= !self.up_next.is_empty();
        self.up_next.clear();
    }

    /// Shuffles the context from the current track on, or goes back to its own order.
    pub fn set_shuffle(&mut self, shuffle: bool) {
        let current = self.order.current().filter(|_| self.started);
        self.order = PlayOrder::new(self.items.len(), shuffle, current);
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
        self.up_next.retain_mut(&mut keep);
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
        self.items.len() + usize::from(self.detour.is_some()) + self.up_next.len()
    }

    /// Everything in play order: the context up to the current item, the up-next item playing,
    /// up next, then the rest of the context.
    pub fn entries(&self) -> Vec<QueueEntry> {
        let entry = |item: &Item, queued: bool| QueueEntry {
            item: item.id,
            locator: item.locator.clone(),
            queued,
        };
        let split = self
            .context_position()
            .map_or(0, |position| self.order.slot_of[position] + 1);
        let context = |slots: &[usize]| {
            slots
                .iter()
                .map(|&position| entry(&self.items[position], false))
                .collect::<Vec<_>>()
        };
        let mut entries = context(&self.order.order[..split]);
        entries.extend(self.detour.iter().map(|item| entry(item, true)));
        entries.extend(self.up_next.iter().map(|item| entry(item, true)));
        entries.extend(context(&self.order.order[split..]));
        entries
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
        let mut slot_of = vec![0; order.len()];
        for (slot, &position) in order.iter().enumerate() {
            slot_of[position] = slot;
        }
        Self {
            order,
            slot_of,
            cursor,
            next_round: None,
            previous_round: None,
            crossing: None,
            changed: true,
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
