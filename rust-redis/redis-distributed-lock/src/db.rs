use std::{collections::BTreeMap, sync::Mutex, thread::sleep, time::Duration};

#[derive(Debug, Clone)]
pub enum Seat {
    Available,
    Sold { fan: String },
}

/// Pretends to be the ticketing database: the source of truth for who owns a seat.
/// Safe to share between threads with `Arc`.
pub struct FakeDb {
    seats: Mutex<BTreeMap<String, Seat>>,
}

impl FakeDb {
    pub fn new(seats: &[&str]) -> Self {
        let seats = seats
            .iter()
            .map(|seat| (seat.to_string(), Seat::Available))
            .collect();
        Self {
            seats: Mutex::new(seats),
        }
    }

    pub fn seat(&self, seat: &str) -> Option<Seat> {
        self.seats
            .lock()
            .expect("seats lock poisoned")
            .get(seat)
            .cloned()
    }

    /// Sells the seat only if nobody bought it first, like
    /// `UPDATE seats SET fan = $1 WHERE id = $2 AND fan IS NULL`.
    /// This is the last guard against selling a seat twice, even when a Redis hold expired.
    pub fn book(&self, seat: &str, fan: &str) -> bool {
        sleep(Duration::from_millis(50));
        let mut seats = self.seats.lock().expect("seats lock poisoned");
        match seats.get_mut(seat) {
            Some(status @ Seat::Available) => {
                *status = Seat::Sold {
                    fan: fan.to_string(),
                };
                true
            }
            _ => false,
        }
    }

    /// Every seat in order, with who bought it.
    pub fn seats(&self) -> Vec<(String, Seat)> {
        self.seats
            .lock()
            .expect("seats lock poisoned")
            .iter()
            .map(|(seat, status)| (seat.clone(), status.clone()))
            .collect()
    }
}
