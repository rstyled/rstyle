pub struct Account {
    balance: u64,
    history: Vec<u64>,
    opened: bool,
}

impl Account {
    pub fn bump(&mut self) {
        self.balance += 1;
    }

    pub fn churn(&mut self, amount: u64) {
        self.balance += amount;
        self.history.push(amount);
        self.opened = true;
        self.bump();
    }

    pub fn peek(&self) -> u64 {
        self.balance
    }
}

pub fn too_many(a: u8, b: u8, c: u8, d: u8, e: u8) -> u8 {
    a + b + c + d + e
}

pub fn spread(a: &mut Vec<u8>, b: &mut Vec<u8>, c: &mut Vec<u8>, d: &mut Vec<u8>) {
    a.push(1);
    b.push(1);
    c.push(1);
    d.push(1);
}

pub fn noisy(path: &str) {
    println!("writing {path}");
    std::fs::write(path, "x").unwrap();
    println!("done");
}

static mut COUNTER: u32 = 0;

pub fn dirty(callback: &dyn Fn(u32)) {
    unsafe {
        COUNTER += 1;
    }
    callback(1);
    let pointer: fn() = || {};
    pointer();
}

// rstyle-allow(fn_params): mirrors a C signature
pub fn allowed(a: u8, b: u8, c: u8, d: u8, e: u8) -> u8 {
    a + b + c + d + e
}

// rstyle-allow(fn_params)
pub fn missing_reason(a: u8, b: u8, c: u8, d: u8, e: u8) -> u8 {
    a + b + c + d + e
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plain {
    pub x: u32,
}

pub fn pure(x: u32) -> u32 {
    let y = x + 1;
    y * 2
}

pub fn very_long() {
    let _v0 = 0;
    let _v1 = 1;
    let _v2 = 2;
    let _v3 = 3;
    let _v4 = 4;
    let _v5 = 5;
    let _v6 = 6;
    let _v7 = 7;
    let _v8 = 8;
    let _v9 = 9;
    let _v10 = 10;
    let _v11 = 11;
    let _v12 = 12;
    let _v13 = 13;
    let _v14 = 14;
    let _v15 = 15;
    let _v16 = 16;
    let _v17 = 17;
    let _v18 = 18;
    let _v19 = 19;
    let _v20 = 20;
    let _v21 = 21;
    let _v22 = 22;
    let _v23 = 23;
    let _v24 = 24;
    let _v25 = 25;
    let _v26 = 26;
    let _v27 = 27;
    let _v28 = 28;
    let _v29 = 29;
    let _v30 = 30;
    let _v31 = 31;
    let _v32 = 32;
    let _v33 = 33;
    let _v34 = 34;
    let _v35 = 35;
    let _v36 = 36;
    let _v37 = 37;
    let _v38 = 38;
    let _v39 = 39;
    let _v40 = 40;
    let _v41 = 41;
    let _v42 = 42;
    let _v43 = 43;
    let _v44 = 44;
    let _v45 = 45;
    let _v46 = 46;
    let _v47 = 47;
    let _v48 = 48;
    let _v49 = 49;
    let _v50 = 50;
    let _v51 = 51;
}
