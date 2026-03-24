use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::{Duration, Instant};

pub enum Event<I> {
    Input(I),
    Tick,
}

pub struct Events {
    rx: std::sync::mpsc::Receiver<Event<KeyEvent>>,
}

impl Events {
    pub fn new(tick_rate: Duration) -> Events {
        let (tx, rx) = std::sync::mpsc::channel();
        let tx_clone = tx.clone();

        std::thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or_else(|| Duration::from_secs(0));

                if event::poll(timeout).expect("no events available") {
                    if let CrosstermEvent::Key(key) = event::read().expect("failed to read event") {
                        if tx_clone.send(Event::Input(key)).is_err() {
                            break;
                        }
                    }
                }

                if last_tick.elapsed() >= tick_rate {
                    if tx_clone.send(Event::Tick).is_err() {
                        break;
                    }
                    last_tick = Instant::now();
                }
            }
        });

        Events { rx }
    }

    pub fn next(&self) -> Result<Event<KeyEvent>, std::sync::mpsc::RecvError> {
        self.rx.recv()
    }
}
