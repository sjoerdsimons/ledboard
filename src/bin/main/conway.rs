// Conway game of live animation

// Point in a W x H grid
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Point<const W: usize, const H: usize>(i16, i16);

impl<const W: usize, const H: usize> Point<W, H> {
    pub fn wrap(&mut self) {
        loop {
            if self.0 < 0 {
                self.0 += W as i16
            } else if self.0 >= W as i16 {
                self.0 -= W as i16
            } else if self.1 < 0 {
                self.1 += H as i16
            } else if self.1 >= H as i16 {
                self.1 -= H as i16
            } else {
                break;
            }
        }
    }
}

impl<const W: usize, const H: usize> core::ops::Add for Point<W, H> {
    type Output = Point<W, H>;

    fn add(self, rhs: Self) -> Self::Output {
        Point(self.0 + rhs.0, self.1 + rhs.1)
    }
}

pub struct GridIter<'a, const W: usize, const H: usize> {
    next: Point<W, H>,
    grid: &'a Grid<W, H>,
}

impl<'a, const W: usize, const H: usize> GridIter<'a, W, H> {
    fn new(grid: &'a Grid<W, H>) -> Self {
        Self {
            next: Point(0, 0),
            grid,
        }
    }
}

impl<const W: usize, const H: usize> Iterator for GridIter<'_, W, H> {
    type Item = (Point<W, H>, bool);

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.1 > H as i16 {
            None
        } else {
            let r = (self.next, self.grid[self.next]);
            self.next.0 += 1;
            if self.next.0 == W as i16 {
                self.next = Point(0, self.next.1 + 1)
            }
            Some(r)
        }
    }
}

#[derive(Eq, PartialEq)]
pub struct Grid<const W: usize, const H: usize> {
    points: [[bool; W]; H],
}

impl<const W: usize, const H: usize> Grid<W, H> {
    pub fn new() -> Self {
        Self {
            points: [[false; W]; H],
        }
    }

    pub fn iter_linear(&self) -> impl Iterator<Item = bool> + '_ {
        self.points.iter().flatten().copied()
    }

    pub fn iter(&self) -> GridIter<'_, W, H> {
        GridIter::new(self)
    }
}

impl<const W: usize, const H: usize> core::ops::Index<Point<W, H>> for Grid<W, H> {
    type Output = bool;

    fn index(&self, mut index: Point<W, H>) -> &Self::Output {
        index.wrap();
        &self.points[index.1 as usize][index.0 as usize]
    }
}

impl<const W: usize, const H: usize> core::ops::IndexMut<Point<W, H>> for Grid<W, H> {
    fn index_mut(&mut self, mut index: Point<W, H>) -> &mut Self::Output {
        index.wrap();
        &mut self.points[index.1 as usize][index.0 as usize]
    }
}

pub struct Conway<const W: usize, const H: usize> {
    current: Grid<W, H>,
}

impl<const W: usize, const H: usize> Conway<W, H> {
    pub fn new(random: u32) -> Self {
        let mut c = Conway {
            current: Grid::new(),
        };
        c.reset(random);
        c
    }

    pub fn reset(&mut self, mut random: u32) {
        for toggle in self.current.points.iter_mut().flatten() {
            if random < 128 {
                random = random.wrapping_mul(13);
            }
            *toggle = random & 0x1 == 0x1;
            random >>= 1;
        }
    }

    pub fn all_dead(&self) -> bool {
        self.current.iter_linear().all(|i| !i)
    }

    fn surrounding(&self, index: Point<W, H>) -> u8 {
        let neighbours = [
            Point(-1, -1),
            Point(0, -1),
            Point(1, -1),
            Point(-1, 0),
            Point(1, 0),
            Point(-1, 1),
            Point(0, 1),
            Point(1, 1),
        ];
        neighbours.iter().fold(0, |acc, n| {
            if self.current[index + *n] {
                acc + 1
            } else {
                acc
            }
        })
    }

    pub fn step(&mut self) -> bool {
        let mut next = Grid::new();

        for (point, alive) in self.current.iter() {
            let surrounding = self.surrounding(point);
            next[point] = match (alive, surrounding) {
                (true, 2 | 3) => true,
                (false, 3) => true,
                _ => false,
            };
        }

        if self.current != next {
            self.current = next;
            true
        } else {
            false
        }
    }

    pub fn iter_linear(&self) -> impl Iterator<Item = bool> + '_ {
        self.current.iter_linear()
    }
}
