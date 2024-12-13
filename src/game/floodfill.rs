use std::collections::{HashSet, VecDeque};

use super::Tile;

type Point = (usize, usize);
type Dir = (i32, i32);

pub struct Floodfill {
    pub regions: Vec<Region>,
}

pub fn get_verts(region: &Region) -> Vec<(usize, usize)> {
    let mut verts = vec![region.start];
    let mut cur = region.start;
    let mut dir = (1, 0);

    let modify = |(x, y): (usize, usize), dir| match dir {
        (1, 0) => (x, y),
        (0, 1) => (x.wrapping_sub(1), y),
        (-1, 0) => (x.wrapping_sub(1), y.wrapping_sub(1)),
        (0, -1) => (x, y.wrapping_sub(1)),
        _ => unreachable!(),
    };

    let is_solid = |p, dir| region.members.contains(&modify(p, dir));

    let mut direction = (1, 0);

    let add = |a: Point, b: Point| (a.0 + b.0, a.1 + b.1);
    let add_dir =
        |a: Point, b: (i32, i32)| ((a.0 as i32 + b.0) as usize, (a.1 as i32 + b.1) as usize);

    let right = |(x, y)| match (x, y) {
        (1, 0) => (0, 1),   // E -> S
        (0, 1) => (-1, 0),  // S -> W
        (-1, 0) => (0, -1), // W -> N
        (0, -1) => (1, 0),  // N -> E
        _ => panic!("right called on {x}, {y}"),
    };

    // let get_next = |p, dir| {};

    let left = |(x, y)| match (x, y) {
        (1, 0) => (0, -1),  // E -> N
        (0, -1) => (-1, 0), // N -> W
        (-1, 0) => (0, 1),  // W -> S
        (0, 1) => (1, 0),   // S -> E
        _ => panic!("left called on {x}, {y}"),
    };
    // let rev = |(x, y)| {
    //     if x != 0 {
    //         (-x, y)
    //     } else if y != 0 {
    //         (x, -y)
    //     } else {
    //         unreachable!("wtf")
    //     }
    // };

    loop {
        if cur == verts[0] && verts.len() > 1 {
            break;
        }
        cur = add_dir(cur, dir);
        if is_solid(cur, left(dir)) {
            // we can go left
            verts.push(cur);
            dir = left(dir);
        } else if is_solid(cur, dir) {
            // we can go forward
        } else {
            verts.push(cur);
            dir = right(dir);
        }
    }

    verts
}

pub struct Region {
    pub start: (usize, usize),
    pub members: HashSet<(usize, usize)>,
}

pub fn floodfill_all(tiles: &Vec<[Tile; 1000]>) -> Floodfill {
    let mut regions: Vec<Region> = Vec::new();

    let lookup = |(x, y): (usize, usize)| -> Tile { tiles[y][x] };

    for x in 0..1000 {
        for y in 0..1000 {
            let p = (x, y);
            if lookup(p).is_solid() {
                if regions.iter().all(|r| !r.members.contains(&p)) {
                    regions.push(fill(p, tiles));
                }
            }
        }
    }

    Floodfill { regions }
}

fn fill(start: (usize, usize), tiles: &Vec<[Tile; 1000]>) -> Region {
    let t0 = std::time::Instant::now();

    let mut to_check = vec![start];
    let mut visited = HashSet::new();

    let lookup = |(x, y): (usize, usize)| -> Tile { tiles[y][x] };

    let mut v = Vec::new();

    let mut i = 0;
    while let Some(p) = to_check.pop() {
        i += 1;
        visited.insert(p);
        neighbors(p, &mut v);

        for &neighbor in &v {
            if !visited.contains(&neighbor) {
                if lookup(neighbor).is_solid() {
                    to_check.push(neighbor);
                }
            }
        }
    }

    Region {
        start,
        members: visited,
    }
}

fn neighbors((x, y): (usize, usize), v: &mut Vec<(usize, usize)>) {
    v.clear();
    if x != 0 {
        v.push((x - 1, y));
    }
    if x != 999 {
        v.push((x + 1, y));
    }
    if y != 0 {
        v.push((x, y - 1));
    }
    if y != 999 {
        v.push((x, y + 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hashset(v: Vec<(usize, usize)>) -> HashSet<(usize, usize)> {
        v.into_iter().collect()
    }

    #[test]
    fn square() {
        let r = Region {
            start: (0, 0),
            members: hashset(vec![(0, 0)]),
        };

        let verts = get_verts(&r);

        assert_eq!(verts, vec![(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)]);
    }

    #[test]
    fn wide() {
        let r = Region {
            start: (0, 0),
            members: hashset(vec![(0, 0), (1, 0), (2, 0)]),
        };

        let verts = get_verts(&r);

        #[rustfmt::skip]
        assert_eq!(verts, vec![(0, 0), (3, 0), (3, 1), (0, 1), (0, 0)]);
    }

    #[test]
    fn tall() {
        let r = Region {
            start: (0, 0),
            members: hashset(vec![(0, 0), (0, 1), (0, 2)]),
        };

        let verts = get_verts(&r);

        assert_eq!(verts, vec![(0, 0), (1, 0), (1, 3), (0, 3), (0, 0)]);
    }

    #[test]
    fn star() {
        #[rustfmt::skip]
        let r = Region {
            start: (2, 0),
            members: hashset(
                vec![
                    /* */           (2, 0),
                    /* */   (1, 1), (2, 1), (3, 1),
                    (0, 2), (1, 2), (2, 2), (3, 2), (4, 2),
                    /* */   (1, 3), (2, 3), (3, 3),
                    /* */           (2, 4)
                ],
            ),
        };

        #[rustfmt::skip]
        let output = vec![
            (2, 0), (3, 0), (3, 1), (4, 1), (4, 2), (5, 2),
            (5, 3), (4, 3), (4, 4), (3, 4), (3, 5), (2, 5),
            (2, 4), (1, 4), (1, 3), (0, 3), (0, 2), (1, 2),
            (1, 1), (2, 1), (2, 0)
        ];

        assert_eq!(get_verts(&r), output);
    }
}
