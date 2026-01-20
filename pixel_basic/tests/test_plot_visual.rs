use pixel_basic::{GameBridge, PixelGameContext, RenderBackend};
use std::cell::RefCell;
use std::rc::Rc;

/// Simple test backend that prints what gets drawn
struct TestBackend {
    draws: Rc<RefCell<Vec<String>>>,
}

impl TestBackend {
    fn new() -> (Self, Rc<RefCell<Vec<String>>>) {
        let draws = Rc::new(RefCell::new(Vec::new()));
        (Self { draws: draws.clone() }, draws)
    }
}

impl RenderBackend for TestBackend {
    fn draw_pixel(&mut self, x: u16, y: u16, ch: char, fg: u8, bg: u8) {
        self.draws.borrow_mut().push(format!("PLOT({}, {}, '{}', {}, {})", x, y, ch, fg, bg));
    }

    fn clear(&mut self) {
        self.draws.borrow_mut().push("CLS".to_string());
    }

    fn add_sprite(&mut self, id: u32, x: i32, y: i32, ch: char, _fg: u8, _bg: u8, _visible: bool) {
        self.draws.borrow_mut().push(format!("SPRITE({}, {}, {}, '{}')", id, x, y, ch));
    }

    fn update_sprite(&mut self, id: u32, x: i32, y: i32, ch: char, _fg: u8, _bg: u8, _visible: bool) {
        self.draws.borrow_mut().push(format!("UPDATE_SPRITE({}, {}, {}, '{}')", id, x, y, ch));
    }

    fn has_sprite(&self, _id: u32) -> bool {
        false
    }
}

#[test]
fn test_plot_renders() {
    let (backend, draws) = TestBackend::new();
    let game_ctx = PixelGameContext::new(backend);
    let mut bridge = GameBridge::new(game_ctx);

    let program = r#"
10 REM Test PLOT rendering - immediate execution
20 PLOT 5, 10, "X", 15, 0
30 PLOT 10, 10, "@", 10, 0
40 PRINT "Done plotting"
50 END
"#;

    bridge.load_program(program).expect("Failed to load");

    // Run program
    for _ in 0..100 {
        match bridge.update(0.016) {
            Ok(false) => break,
            Ok(true) => {},
            Err(e) => panic!("Error: {:?}", e),
        }
    }

    let draws = draws.borrow();
    println!("Draws: {:?}", draws);

    // Should have at least 2 PLOT calls
    assert!(draws.len() >= 2, "Expected at least 2 draws, got {}", draws.len());
    assert!(draws.iter().any(|s| s.contains("PLOT(5, 10")), "Missing first PLOT");
    assert!(draws.iter().any(|s| s.contains("PLOT(10, 10")), "Missing second PLOT");
}
