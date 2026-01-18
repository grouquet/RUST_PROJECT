use std::io::{self, Write};
use std::time::{Duration, Instant};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute, queue,
    style::Print,
    terminal::{self, Clear, ClearType},
};

#[derive(Clone, Copy, Eq, PartialEq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Thing {
    Wall,
    Pellet,
    Empty
}
struct Game {
    pacman_pos: Position, // position du Pacman
    width: i32, // largeur de la grille
    length: i32, // longueur de la grille
    pacman_dir: Position, // direction du Pacman
    map: Vec<Vec<Thing>>, // la carte du jeu
    score : u32, // le score du joueur
    pellets_left: u32, // pastilles restantes -> pour gérer la victoire
    wanted_dir: Position, // direction voulue par le joueur -> pour conserver la direction si on peut pas tourner
    ghost_pos: Position, // position du fantôme
    game_over: bool, // état de fin de jeu
    tick_count: u64, // compteur de ticks pour deplacer fantome (1tick sur 2)
    ghost_dir: Position, // direction du fantôme, pour éviter le demi-tour, une fois qu'on met dist manhattan
}

impl Game {
    //on crée une nouvelle instance de jeu à parir d'une map lisible
    fn from_ascii(ascii: &[&str]) -> Self {
        let length = ascii.len() as i32;
        let width = ascii[0].chars().count() as i32;

        let mut map = vec![vec![Thing::Empty; width as usize]; length as usize]; //initialisier grille vide
        let mut pacman_pos = Position { x: 0, y: 0 }; // initialiser position pacman
        let mut wanted_dir = Position { x: 0, y: 0 }; // initialiser direction voulue
        let mut ghost_pos = Position { x: 1, y: 1 }; // initialiser position fantôme

        let mut pellets_left: u32 = 0;

        for (y, line) in ascii.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                map[y][x] = match ch {
                    '#' => Thing::Wall,
                    '.' => {
                        pellets_left += 1;
                        Thing::Pellet
                    },
                    'P' => {
                        pacman_pos = Position { x: x as i32, y: y as i32 };
                        Thing::Empty
                    },
                    'G' => {
                        ghost_pos = Position { x: x as i32, y: y as i32 };
                        Thing::Empty
                    },
                    _ => Thing::Empty,
                };
            }
        }

        Self {
            pacman_pos,
            width,
            length,
            pacman_dir: Position { x: 0, y: 0 },
            map,
            score: 0,
            pellets_left,
            wanted_dir,
            ghost_pos,
            game_over: false,
            tick_count: 0,
            ghost_dir: Position { x: 0, y: 0 },
        }
    }

    fn in_bounds(&self, pos: Position) -> bool {
        pos.x >= 0 && pos.x < self.width && pos.y >= 0 && pos.y < self.length
    }

    //donne le type à une position donnée
    fn thing(&self, pos: Position) -> Thing {
        self.map[pos.y as usize][pos.x as usize]
    }

    //met un type à une position donnée
    fn set_thing(&mut self, pos: Position, thing: Thing) {
        self.map[pos.y as usize][pos.x as usize] = thing;
    }

    // Gère les entrées clavier pour changer la direction du Pacman
    fn handle_input(&mut self, code: KeyCode) {
        self.wanted_dir = match code {
            KeyCode::Up | KeyCode::Char('z') => Position { x: 0, y: -1 }, // haut
            KeyCode::Down | KeyCode::Char('s') => Position { x: 0, y: 1 }, // bas
            KeyCode::Left | KeyCode::Char('q') => Position { x: -1, y: 0 }, // gauche
            KeyCode::Right | KeyCode::Char('d') => Position { x: 1, y: 0 }, // droite
            _ => self.wanted_dir, // on ne reset pas !
        };
    }

    fn next_position(&self, from: Position, dir: Position) -> Position {
        Position {
            x: from.x + dir.x,
            y: from.y + dir.y,
        }
    }

    fn can_move(&self, from: Position, dir: Position) -> bool {
        if dir.x == 0 && dir.y == 0 {
            return false; // pas de mouvement
        }
        let next = self.next_position(from, dir);
        self.in_bounds(next) && self.thing(next) != Thing::Wall
    }
    
    fn manhattan_distance(a: Position, b: Position) -> i32 {
        (a.x - b.x).abs() + (a.y - b.y).abs()
    }

    fn opposite(dir: Position) -> Position {
        Position { x: -dir.x, y: -dir.y }
    }

    fn move_ghost(&mut self, tick: u64) {
        // Logique de déplacement du fantôme (à implémenter)
        let dirs = [
            Position { x: 0, y: -1 },
            Position { x: 0, y: 1 },
            Position { x: -1, y: 0 },
            Position { x: 1, y: 0 },
        ];

        let mut candidates: Vec<Position> = Vec::new();
        for dir in dirs.iter() {
            if self.can_move(self.ghost_pos, *dir) {
                candidates.push(*dir);
            }
        }

        if candidates.is_empty() {
            return; // pas de mouvement possible
        }

        let opp = Self::opposite(self.ghost_dir);
        let mut filtered: Vec<Position> = if self.ghost_dir.x == 0 && self.ghost_dir.y == 0 {
            candidates.clone() // pas de direction précédente, on garde toutes les options
        } else {
            candidates
                .iter()
                .copied()
                .filter(|&d| d != opp)
                .collect()
        };

        if filtered.is_empty() {
            filtered = candidates; // autoriser le demi-tour si nécessaire
        }


        let mut best_dir = filtered[0];
        let mut best_distance = i32::MAX;

        for dir in filtered {

            let next = self.next_position(self.ghost_pos, dir);
            let dist = Self::manhattan_distance(next, self.pacman_pos);

            if dist < best_distance {
                best_distance = dist;
                best_dir = dir;
            }

        }
        self.ghost_pos = self.next_position(self.ghost_pos, best_dir);
        self.ghost_dir = best_dir;
    }

    // Met à jour la position du Pacman en fonction de sa direction
    fn update(&mut self) {

        if self.game_over || self.pellets_left == 0 {
            return; // ne fait rien si le jeu est terminé
        }

        //si la direction voulue est possible, on la prend
        if self.can_move(self.pacman_pos, self.wanted_dir) {
            self.pacman_dir = self.wanted_dir; // met à jour la direction voulue
        }


        //sinon on garde la direction actuelle
        if !self.can_move(self.pacman_pos, self.pacman_dir) {
            return; // ne bouge pas si la direction actuelle est bloquée
        }

        let next = self.next_position(self.pacman_pos, self.pacman_dir);

        self.pacman_pos = next; //déplacement du pacman
        
        let tick = (self.score as u64)
            .wrapping_add(self.pacman_pos.x as u64)
            .wrapping_add((self.pacman_pos.y as u64) << 8);


        self.tick_count = self.tick_count.wrapping_add(1);
        
        if self.tick_count % 2 == 0 {
            self.move_ghost(tick); // déplace le fantôme tous les 2 ticks
        };

        //collision
        if self.pacman_pos == self.ghost_pos {
            self.game_over = true; // fin du jeu
            return;
        }

        if self.thing(next) == Thing::Pellet {
            self.score += 1; // incrémente le score
            self.pellets_left = self.pellets_left.saturating_sub(1); // décrémente les pastilles restantes
            self.set_thing(next, Thing::Empty); // enlève le pellet
        }
    }

    fn render (&self, out: &mut impl Write) -> io::Result<()>  {

        queue!(out, cursor::MoveTo(0, 0))?;  

        queue!(out, Print(format!("Score: {}\r\n  |  Pellets left: {}\r\n", self.score, self.pellets_left)))?;

        if self.game_over {
            queue!(out, Print("Game Over! Press 'Esc' or 'x' to exit.\r\n"))?;
        } else if self.pellets_left == 0 {
            queue!(out, Print("You win! Press 'Esc' or 'x' to exit.\r\n"))?;
        } else {
            queue!(out, Print("\r\n"))?; // ligne vide entre le score et la grille
        }
        //Dessin --> boucles imbriquées 
        for y in 0..self.length {
            for x in 0..self.width {
                let pos = Position { x, y };
                let pacman = if pos == self.pacman_pos {
                    "C" // dessine Pacman
                } else if pos == self.ghost_pos {
                    "G" // dessine le fantôme
                } else {
                    match self.thing(pos) {
                        Thing::Wall => "#",    // dessine un mur
                        Thing::Pellet => ".",  // dessine un pellet
                        Thing::Empty => " "    // dessine un espace vide
                    }
                };
                queue!(out, Print(pacman))?;
            }
            queue!(out, Print("\r\n"))?;
        }
        out.flush()?;
        Ok(())
    }
}

fn main() -> io::Result<()> {

    const MAP: &[&str] = &[
        "########################################",
        "#P..............##..............G......#",
        "#.####.#####.###.##.###.#####.####.###.#",
        "#......#...#................#...#......#",
        "######.#.#.########.##.########.#.######",
        "#......#.#....#.....##.....#....#......#",
        "#.##########.#.###.####.###.#.########.#",
        "#............#..............#..........#",
        "#.##########.#####.##.#####.##########.#",
        "#......#...........##...........#......#",
        "######.#.#########.##.#########.#.######",
        "#......#.....#........#.....#...#......#",
        "#.##########.#.######.######.#.#########",
        "#..........#.#..............#.#........#",
        "#.########.#.#######.######.#.########.#",
        "#......G...#.....##.....##..#..........#",
        "########################################",
    ];



    //preparation du terminal
    terminal::enable_raw_mode()?; // pas de line buffering
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    //jeu
    let mut game = Game::from_ascii(MAP); // jeu à partir de MAP

    //tick rate
    let frame_duration = Duration::from_millis(50);
    let mut last = Instant::now();

    //boucle principale
    'game_loop: loop {
        //temps restant avant la prochaine frame
        let elapsed = last.elapsed();
        let timeout = frame_duration.saturating_sub(elapsed);

        //gestion des entrées
        if event::poll(timeout)? {
            if let Event::Key(event) = event::read()? {
                if event.code == KeyCode::Esc || event.code == KeyCode::Char('x') {
                    break 'game_loop; // quitter le jeu
                }
                if event.code == KeyCode::Char('r') {
                    game = Game::from_ascii(MAP); // redémarrer le jeu
                    continue 'game_loop;
                }
                game.handle_input(event.code);
            }
        }

        

        //update à chaque tick
        if last.elapsed() >= frame_duration {
            game.update();
            game.render(&mut stdout)?;
            last = Instant::now();
        }
    }

    //restauration du terminal
    execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
