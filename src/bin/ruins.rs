const COINS: [usize; 5] = [2, 3, 5, 7, 9];
const COIN_NAMES: [&str; 10] = [
    "",
    "",
    "red coin",
    "corroded coin",
    "",
    "shiny coin",
    "",
    "concave coin",
    "",
    "blue coin",
];
const TARGET: usize = 399;

fn find_permutation(arr: &mut [usize; 5], mut predicate: impl FnMut(&[usize; 5]) -> bool) {
    let mut p = (0..=arr.len()).collect::<Vec<_>>();

    if predicate(&*arr) {
        return;
    }

    let mut idx = 1;
    while idx < arr.len() {
        p[idx] -= 1;
        let j = if idx % 2 == 1 { p[idx] } else { 0 };
        arr.swap(idx, j);

        if predicate(&*arr) {
            return;
        }

        idx = 1;
        while p[idx] == 0 {
            p[idx] = idx;
            idx += 1;
        }
    }

    predicate(&*arr);
}

fn main() {
    // _ + _ * _^2 + _^3 - _ = 399

    let mut coins = COINS;

    find_permutation(&mut coins, |&[a, b, c, d, e]| {
        a + b * c * c + d * d * d - e == TARGET
    });

    let mut coins = coins.iter().copied();
    print!("{:?}", COIN_NAMES[coins.next().unwrap()]);
    for coin in coins {
        print!(" {:?}", COIN_NAMES[coin]);
    }
}
