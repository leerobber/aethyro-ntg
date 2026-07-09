//! State-slot management: mutable agent/node state with parent-offset lineage.
//!
//! Inspired by ChronosLedger's fast, append-only mmap state store, but
//! without the false "tamper-evident ledger" claim. This is a real, fast
//! way to track evolving agent state (desires, fitness, maturity,
//! generation) and maintain parent lineage for replay/recovery.
//!
//! Each slot is 48 bytes (6 u64 fields): agent_id, desires, fitness_int,
//! parent_offset, generation, timestamp. Updates are append-only by
//! convention; mutation is tracked in the main ledger, not here.

use super::NtgError;
use std::collections::HashMap;

/// A single state slot: immutable once written, logically chained via
/// parent_offset to the previous version of this agent's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateSlot {
    pub agent_id: u32,
    /// Desire signal (arbitrary i32 encoding of goal/preference)
    pub desires: i32,
    /// Fitness encoded as integer (microseconds of latency, for now)
    pub fitness_int: u64,
    /// Byte offset of the previous version of this agent's state (0 = genesis)
    pub parent_offset: u64,
    /// Generation counter (incremented each time this agent mutates)
    pub generation: u32,
    /// Creation timestamp
    pub timestamp: u64,
}

impl StateSlot {
    const SIZE_BYTES: usize = 48; // 6 * 8 bytes

    pub fn to_bytes(&self) -> [u8; Self::SIZE_BYTES] {
        let mut out = [0u8; Self::SIZE_BYTES];
        out[0..4].copy_from_slice(&self.agent_id.to_le_bytes());
        out[4..8].copy_from_slice(&(self.desires as u32).to_le_bytes());
        out[8..16].copy_from_slice(&self.fitness_int.to_le_bytes());
        out[16..24].copy_from_slice(&self.parent_offset.to_le_bytes());
        out[24..28].copy_from_slice(&self.generation.to_le_bytes());
        out[28..36].copy_from_slice(&self.timestamp.to_le_bytes());
        out
    }

    pub fn from_bytes(bytes: &[u8; Self::SIZE_BYTES]) -> Result<Self, NtgError> {
        // Fixed-width slices; try_into is infallible for these lengths.
        let agent_id = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let desires = i32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let fitness_int = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        let parent_offset = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
        let generation = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
        let timestamp = u64::from_le_bytes(bytes[28..36].try_into().unwrap());

        Ok(Self {
            agent_id,
            desires,
            fitness_int,
            parent_offset,
            generation,
            timestamp,
        })
    }
}

/// State slot store: append-only, in-memory simulation of mmap state store.
/// Phase 3 uses in-memory; Phase 3.1+ can mmap to a real file.
#[derive(Clone, Debug)]
pub struct StateSlotStore {
    /// Slots in append order
    slots: Vec<StateSlot>,
    /// Index of latest slot for each agent_id (for fast lookup)
    agent_latest: HashMap<u32, usize>,
    /// Optional file path for future mmap implementation
    _filepath: Option<String>,
}

impl StateSlotStore {
    pub fn new(filepath: Option<&str>) -> Result<Self, NtgError> {
        Ok(Self {
            slots: Vec::new(),
            agent_latest: HashMap::new(),
            _filepath: filepath.map(String::from),
        })
    }

    /// Append a new state slot, updating the agent's lineage pointer.
    /// The slot's parent_offset points to the previous version (or 0 = genesis).
    pub fn write_slot(&mut self, slot: StateSlot) -> Result<usize, NtgError> {
        let offset = self.slots.len();

        // For lineage: find the previous version of this agent's state.
        // parent_offset is 1-based (prev_idx + 1) so 0 stays reserved for genesis.
        // (0 as a raw index would collide with the first slot.)
        let corrected_slot = if let Some(&prev_idx) = self.agent_latest.get(&slot.agent_id) {
            StateSlot {
                parent_offset: (prev_idx as u64) + 1,
                generation: slot.generation,
                ..slot
            }
        } else {
            // First time seeing this agent: parent_offset = 0 (genesis)
            StateSlot {
                parent_offset: 0,
                ..slot
            }
        };

        self.slots.push(corrected_slot);
        self.agent_latest.insert(slot.agent_id, offset);
        Ok(offset)
    }

    /// Get the latest state for an agent.
    pub fn get_latest(&self, agent_id: u32) -> Option<(usize, StateSlot)> {
        self.agent_latest
            .get(&agent_id)
            .and_then(|&idx| self.slots.get(idx).map(|slot| (idx, *slot)))
    }

    /// Get a specific slot by offset.
    pub fn get_slot(&self, offset: usize) -> Option<StateSlot> {
        self.slots.get(offset).copied()
    }

    /// Trace the lineage of an agent: walk backwards via parent_offset.
    pub fn lineage(&self, agent_id: u32) -> Result<Vec<StateSlot>, NtgError> {
        let mut lineage = Vec::new();
        let mut current_idx = *self
            .agent_latest
            .get(&agent_id)
            .ok_or(NtgError::InvalidInput(format!(
                "Agent {} not found",
                agent_id
            )))?;

        loop {
            let slot = self.slots[current_idx];
            lineage.push(slot);

            if slot.parent_offset == 0 {
                // Reached genesis
                break;
            }

            // Decode 1-based parent pointer
            current_idx = (slot.parent_offset - 1) as usize;
            if current_idx >= self.slots.len() {
                return Err(NtgError::InvalidInput(format!(
                    "Lineage pointer out of bounds: {} >= {}",
                    current_idx,
                    self.slots.len()
                )));
            }
        }

        lineage.reverse(); // Return oldest-first
        Ok(lineage)
    }

    /// Verify that all parent_offset pointers are valid (lineage integrity).
    pub fn verify_lineage(&self) -> Result<(), String> {
        for (idx, slot) in self.slots.iter().enumerate() {
            if slot.parent_offset > 0 {
                let parent = (slot.parent_offset - 1) as usize;
                if parent >= idx {
                    return Err(format!(
                        "Slot {} has invalid parent_offset: {}",
                        idx, slot.parent_offset
                    ));
                }
            }
        }
        Ok(())
    }

    /// Number of slots stored.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Number of unique agents.
    pub fn agent_count(&self) -> usize {
        self.agent_latest.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_slot_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let slot = StateSlot {
            agent_id: 42,
            desires: 100,
            fitness_int: 5000,
            parent_offset: 0,
            generation: 1,
            timestamp: 1000,
        };
        let bytes = slot.to_bytes();
        let restored = StateSlot::from_bytes(&bytes)?;
        assert_eq!(slot, restored);
        Ok(())
    }

    #[test]
    fn write_and_retrieve_slot() -> Result<(), NtgError> {
        let mut store = StateSlotStore::new(None)?;
        let slot = StateSlot {
            agent_id: 1,
            desires: 50,
            fitness_int: 3000,
            parent_offset: 0,
            generation: 1,
            timestamp: 1000,
        };

        let offset = store.write_slot(slot)?;
        assert_eq!(offset, 0);
        assert_eq!(store.get_slot(0), Some(slot));
        Ok(())
    }

    #[test]
    fn agent_lineage_tracks_generations() -> Result<(), NtgError> {
        let mut store = StateSlotStore::new(None)?;

        // Write generation 1
        let slot1 = StateSlot {
            agent_id: 1,
            desires: 50,
            fitness_int: 3000,
            parent_offset: 0,
            generation: 1,
            timestamp: 1000,
        };
        store.write_slot(slot1)?;

        // Write generation 2 (should auto-link to gen 1)
        let slot2 = StateSlot {
            agent_id: 1,
            desires: 60,
            fitness_int: 3100,
            parent_offset: 0, // Will be corrected to 0
            generation: 2,
            timestamp: 1100,
        };
        store.write_slot(slot2)?;

        // Write generation 3
        let slot3 = StateSlot {
            agent_id: 1,
            desires: 70,
            fitness_int: 3200,
            parent_offset: 0,
            generation: 3,
            timestamp: 1200,
        };
        store.write_slot(slot3)?;

        let lineage = store.lineage(1)?;
        assert_eq!(lineage.len(), 3);
        assert_eq!(lineage[0].generation, 1);
        assert_eq!(lineage[1].generation, 2);
        assert_eq!(lineage[2].generation, 3);
        Ok(())
    }

    #[test]
    fn lineage_for_multiple_agents() -> Result<(), NtgError> {
        let mut store = StateSlotStore::new(None)?;

        store.write_slot(StateSlot {
            agent_id: 1,
            desires: 50,
            fitness_int: 3000,
            parent_offset: 0,
            generation: 1,
            timestamp: 1000,
        })?;

        store.write_slot(StateSlot {
            agent_id: 2,
            desires: 60,
            fitness_int: 3100,
            parent_offset: 0,
            generation: 1,
            timestamp: 1100,
        })?;

        store.write_slot(StateSlot {
            agent_id: 1,
            desires: 55,
            fitness_int: 3050,
            parent_offset: 0,
            generation: 2,
            timestamp: 1200,
        })?;

        assert_eq!(store.agent_count(), 2);
        assert_eq!(store.lineage(1)?.len(), 2);
        assert_eq!(store.lineage(2)?.len(), 1);
        Ok(())
    }

    #[test]
    fn verify_lineage_detects_invalid_pointers() -> Result<(), NtgError> {
        let mut store = StateSlotStore::new(None)?;

        store.write_slot(StateSlot {
            agent_id: 1,
            desires: 50,
            fitness_int: 3000,
            parent_offset: 0,
            generation: 1,
            timestamp: 1000,
        })?;

        // Manually inject a bad slot (1-based parent that points past current idx)
        store.slots.push(StateSlot {
            agent_id: 2,
            desires: 60,
            fitness_int: 3100,
            parent_offset: 999, // Invalid: decoded parent >= idx
            generation: 1,
            timestamp: 1100,
        });

        assert!(store.verify_lineage().is_err());
        Ok(())
    }

    #[test]
    fn get_latest_returns_most_recent() -> Result<(), NtgError> {
        let mut store = StateSlotStore::new(None)?;

        let slot1 = StateSlot {
            agent_id: 1,
            desires: 50,
            fitness_int: 3000,
            parent_offset: 0,
            generation: 1,
            timestamp: 1000,
        };
        store.write_slot(slot1)?;

        let slot2 = StateSlot {
            agent_id: 1,
            desires: 55,
            fitness_int: 3050,
            parent_offset: 0,
            generation: 2,
            timestamp: 1100,
        };
        store.write_slot(slot2)?;

        let (_, latest) = store.get_latest(1).unwrap();
        assert_eq!(latest.generation, 2);
        assert_eq!(latest.desires, 55);
        Ok(())
    }
}
