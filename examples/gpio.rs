use jetsongpio::{Direction, GPIO, Level, Mode};
use std::thread;
use std::time::Duration;

const PIN: u32 = 29;

fn main() {
    println!("Starting GPIO test on PIN {}...", PIN);

    let mut gpio = GPIO::new();
    gpio.setmode(Mode::BOARD).unwrap();

    // Setup PIN as output with initial LOW value
    gpio.setup(vec![PIN], Direction::OUT, Some(Level::LOW), None)
        .unwrap();
    println!("PIN {} set to output mode", PIN);

    let mut count = 0;
    loop {
        // Set HIGH
        gpio.output(vec![PIN], vec![Level::HIGH]).unwrap();
        println!("[{}] PIN {}: HIGH", count, PIN);

        // Wait 1 second
        thread::sleep(Duration::from_secs(1));

        // Set LOW
        gpio.output(vec![PIN], vec![Level::LOW]).unwrap();
        println!("[{}] PIN {}: LOW", count, PIN);

        // Wait 1 second
        thread::sleep(Duration::from_secs(1));

        count += 1;

        // Break after 10 iterations for safety
        if count >= 10 {
            break;
        }
    }

    // Cleanup
    gpio.cleanup(Some(vec![PIN])).unwrap();
    println!("GPIO test completed and cleaned up.");
}
