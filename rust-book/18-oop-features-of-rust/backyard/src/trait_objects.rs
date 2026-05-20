trait Draw {
    fn draw(&self) {
        println!("draw");
    }
}

struct ScreenTraitObject {
    components: Vec<Box<dyn Draw>>,
}

impl ScreenTraitObject {
    fn new() -> ScreenTraitObject {
        ScreenTraitObject { components: vec![] }
    }

    fn add(&mut self, component: Box<dyn Draw>) {
        self.components.push(component);
    }

    fn run(&self) {
        for component in self.components.iter() {
            component.draw();
        }
    }
}

struct ScreenGenerics<T: Draw> {
    components: Vec<T>,
}

impl<T> ScreenGenerics<T> where T: Draw {
    fn new() -> ScreenGenerics<T> {
        ScreenGenerics { components: vec![] }
    }

    fn add(&mut self, component: T) {
        self.components.push(component);
    }

    fn run(&self) {
        for component in self.components.iter() {
            component.draw();
        }
    }
}

struct Button {
    width: u32,
    height: u32,
    label: String,
}

impl Button {
    fn new(width: u32, height: u32, label: &str) -> Button {
        Button { width, height, label: label.to_string() }
    }
}

impl Draw for Button {
    fn draw(&self) {
        println!("draw Button with width {}, height {} and label {}", self.width, self.height, self.label);
    }
}

struct SelectBox {
    width: u32,
    height: u32,
    options: Vec<String>,
}

impl Draw for SelectBox {
    fn draw(&self) {
        println!("draw SelectBox with width {} and height {} and options: {:?}", self.width, self.height, self.options);
    }
}

pub fn run() {
    let mut screen1 = ScreenTraitObject::new();
    let mut screen2: ScreenGenerics<Button> = ScreenGenerics::new();

    let button1 = Button::new(10, 10, "Hello");
    let button2 = Button::new(20, 20, "World");

    screen1.add(Box::new(button1));
    screen2.add(button2);

    screen1.run();
    screen2.run();

    let screen3 = ScreenTraitObject {
        components: vec![
            Box::new(SelectBox {
                width: 75,
                height: 10,
                options: vec![
                    String::from("Yes"),
                    String::from("Maybe"),
                    String::from("No"),
                ],
            }),
            Box::new(Button {
                width: 50,
                height: 10,
                label: String::from("OK"),
            }),
        ],
    };

    screen3.run();
}