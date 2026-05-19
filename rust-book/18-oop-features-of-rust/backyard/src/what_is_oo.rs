pub struct AverageCollection {
    list: Vec<i32>,
    average: f64,
}

impl AverageCollection {
    pub fn new() -> AverageCollection {
        AverageCollection { list: Vec::new(), average: 0.0 }
    }

    pub fn add(&mut self, value: i32) {
        self.list.push(value);
        self.update_average();
    }

    pub fn remove(&mut self) -> Option<i32> {
        let result = self.list.pop();
        match result {
            Some(value) => {
                self.update_average();
                Some(value)
            },
            None => None,
        }
    }

    fn update_average(&mut self) {
        let total = self.list.iter().sum::<i32>() as f64;
        self.average = total / self.list.len() as f64;
    }

    pub fn average(&self) -> f64 {
        self.average
    }
}

pub fn run() {
    let mut c = AverageCollection::new();
    c.add(1);
    c.add(2);
    c.add(3);
    println!("average: {}", c.average());
}