#[macro_use]
extern crate lazy_static;
extern crate rand;

use hlt::command::Command;
use hlt::direction::Direction;
use hlt::game::Game;
use hlt::log::Log;
use hlt::navi::Navi;
use hlt::position::Position;
use rand::Rng;
use rand::SeedableRng;
use rand::XorShiftRng;
use std::env;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use std::collections::HashMap;
use std::time;

mod hlt;

#[derive(Eq, PartialEq, Debug)]
enum ShipStates {
Exploring,
Returning,
Mining,
SettlingDropoff,
GoTo
}



fn main() {
    let args: Vec<String> = env::args().collect();
    /*
    let rng_seed: u64 = if args.len() > 1 {
        args[1].parse().unwrap()
    } else {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    };*/
    let rng_seed: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let seed_bytes: Vec<u8> = (0..16).map(|x| ((rng_seed >> (x % 8)) & 0xFF) as u8).collect();
    let mut rng: XorShiftRng = SeedableRng::from_seed([
        seed_bytes[0], seed_bytes[1], seed_bytes[2], seed_bytes[3],
        seed_bytes[4], seed_bytes[5], seed_bytes[6], seed_bytes[7],
        seed_bytes[8], seed_bytes[9], seed_bytes[10], seed_bytes[11],
        seed_bytes[12], seed_bytes[13], seed_bytes[14], seed_bytes[15]
    ]);

    let mut ship_status = HashMap::new();
    let mut game = Game::new();
    let mut navi = Navi::new(game.map.width, game.map.height);
    // At this point "game" variable is populated with initial map data.
    // This is a good place to do computationally expensive start-up pre-processing.
    // As soon as you call "ready" function below, the 2 second per turn timer will start.
    let game_length = match game.map.width {
        40 => 426,
        32 => 401,
        48 => 451,
        56 => 476,
        64 => 501,
        _ => 450
    };
    let mut filter_divider = 3;
    let mut dropoff_turn : usize = 150;
    let mut reserve_dropoff_halite = 18;
    let mut dropoff_distance_penalty = 78;
    let mut dropoff_group_send = 12;
    let mut search_radius : i32 = 5;
    let mut random_death_turn = 20;
    let mut exploring_move_multiplier : usize = 130;
    let mut stop_producing_ships_turn : usize = 170;
    
    if game.players.len() == 2 { 
        if game.map.width == 32 {
            filter_divider = 2;
            dropoff_turn = 176;
            reserve_dropoff_halite = 0;
            dropoff_distance_penalty = 97;
            dropoff_group_send = 0;
            search_radius = 1;
            random_death_turn = 12;
            exploring_move_multiplier = 163;
            stop_producing_ships_turn = 192;
        }
        if game.map.width == 48 {
            filter_divider = 2;
            dropoff_turn = 209;
            reserve_dropoff_halite = 4;
            dropoff_distance_penalty = 69;
            dropoff_group_send = 3;
            search_radius = 1;
            random_death_turn = 17;
            exploring_move_multiplier = 143;
            stop_producing_ships_turn = 174;
        }
        if game.map.width == 64 {
            filter_divider = 3;
            dropoff_turn = 140;
            reserve_dropoff_halite = 3;
            dropoff_distance_penalty = 27;
            dropoff_group_send = 12;
            search_radius = 2;
            random_death_turn = 25;
            exploring_move_multiplier = 147;
            stop_producing_ships_turn = 204;
        }

    }
    if game.players.len() == 4 { 
        if game.map.width == 32 {
            filter_divider = 1;
            dropoff_turn = 122;
            reserve_dropoff_halite = 0;
            dropoff_distance_penalty = 64;
            dropoff_group_send = 1;
            search_radius = 1;
            random_death_turn = 21;
            exploring_move_multiplier = 145;
            stop_producing_ships_turn = 179;
        }
        if game.map.width == 48 {
            filter_divider = 2;
            dropoff_turn = 173;
            reserve_dropoff_halite = 16;
            dropoff_distance_penalty = 79;
            dropoff_group_send = 9;
            search_radius = 1;
            random_death_turn = 19;
            exploring_move_multiplier = 160;
            stop_producing_ships_turn = 203;
        }
        if game.map.width == 64 {
            filter_divider = 3;
            dropoff_turn = 132;
            reserve_dropoff_halite = 21;
            dropoff_distance_penalty = 53;
            dropoff_group_send = 9;
            search_radius = 4;
            random_death_turn = 26;
            exploring_move_multiplier = 133;
            stop_producing_ships_turn = 180;
        }

    }

    if args.len() > 9 {
        filter_divider = args[1].parse().unwrap();
        dropoff_turn = args[2].parse().unwrap();
        reserve_dropoff_halite = args[3].parse().unwrap();
        dropoff_distance_penalty = args[4].parse().unwrap();
        dropoff_group_send = args[5].parse().unwrap();
        search_radius = args[6].parse().unwrap();
        random_death_turn = args[7].parse().unwrap();
        exploring_move_multiplier = args[8].parse().unwrap();
        stop_producing_ships_turn = args[9].parse().unwrap();
    }
     

    let exploring_move_multiplier: f32 = exploring_move_multiplier as f32 / 100.0;


    let filter_radius = game.map.width/filter_divider;

    let mut shipyard_unavalible_steps = 0;
    let mut dropoff_creating = 0;
    let mut ship_id_for_dropoff = hlt::ShipId(99999);
    let mut minimum_distance_to_dropoff = 9999; 
    

    let search_start = time::Instant::now();
    let mut possible_cells_list : Vec<Vec<isize>> = Vec::new();
    for map_x in 0..game.map.width {
        for map_y in 0..game.map.height {
            let mut halite_sum = 0;
            for x in -search_radius..search_radius {
                for y in -search_radius..search_radius {
                    halite_sum += game.map.at_position(&Position{x:(map_x as i32 + x), 
                                                                 y:(map_y as i32 + y)}).halite as isize;
                }
            }
            let distance_penalty = dropoff_distance_penalty * game.map.calculate_distance(&game.players[game.my_id.0].shipyard.position, &Position{x: map_x as i32, y: map_y as i32});
            halite_sum -= distance_penalty as isize * ((distance_penalty as f32).log2()) as isize;
            possible_cells_list.push(vec![map_x as isize , map_y as isize, halite_sum.clone()]);
        }
    }

    possible_cells_list.sort_by(|b, a| a[2].cmp(&b[2]));
    possible_cells_list = possible_cells_list.into_iter().filter(|x| {
        for player in &game.players{
            if game.map.calculate_distance(&player.shipyard.position, &Position{x: x[0] as i32,y: x[1] as i32}) < filter_radius {
                return false;
            }
        }
        true
    }).collect();

    Log::log(&format!("Map possibilities calculated in:  {}.", search_start.elapsed().as_secs() as f64
           + search_start.elapsed().subsec_millis() as f64 * 1e-3));

    Game::ready("MyRustBot");

    Log::log(&format!("Successfully created bot! My Player ID is {}. Bot rng seed is {}.", game.my_id.0, rng_seed));
    Log::log(&format!("dropoff turn: {}", dropoff_turn));
    Log::log(&format!("args vector: {:?}", args));
    Log::log(&format!("filter radius: {:?}", filter_radius));
    Log::log(&format!("filter divider: {:?}", filter_divider));

    let mut ships_went_to_dropoff : Vec<usize> = Vec::new();

    loop {
        game.update_frame();
        navi.update_frame(&game);
        let backup_navi = navi.clone();

        let me = &game.players[game.my_id.0];
        let map = &mut game.map;
        
        let mut command_queue: Vec<Command> = Vec::new();

        let divider = match game.turn_number {
            1..=20 => 2,
            21..=40 => 3,
            41..=60 => 5,
            61..=100 => 6,
            101..=200 => 7,
            _ => 20
        };

        let return_minimum = match game.turn_number {
            1..=100 => 1.0,
            101..=350 => 0.95,
            _ => 0.9,
        };
        
        for ship in &game.ships {
            if ship.1.position.x == me.shipyard.position.x && ship.1.position.y == me.shipyard.position.y && ship.1.owner != game.my_id {
                navi.mark_safe(&me.shipyard.position);
            }
        }
        
        let mut shipyard_surrounding = me.shipyard.position.get_surrounding_cardinals();
        shipyard_surrounding.push(me.shipyard.position);

        let mut exit_avalible = false;
        for possible_position in shipyard_surrounding {
            if navi.is_safe(&possible_position){
                exit_avalible = true;
            }
        };
        if exit_avalible == false {
            shipyard_unavalible_steps += 1;
        }
        if shipyard_unavalible_steps > 4 {
            shipyard_unavalible_steps = 0;
            navi.mark_safe(&me.shipyard.position);
            navi.mark_safe(&me.shipyard.position.directional_offset(Direction::West));
        }

        if !game.ships.contains_key(&ship_id_for_dropoff) && dropoff_creating == 1{
            Log::log("dropoff ship dead, finding new candidate");
            dropoff_creating = 0;
        }
        if dropoff_creating == 1{
            Log::log(&format!("Dropoff ship info: {:?}.", game.ships[&ship_id_for_dropoff])); 
        }

        if game.turn_number > dropoff_turn && dropoff_creating == 0 && possible_cells_list.len() > 0{
            for ship_id in &me.ship_ids {
                let ship = &game.ships[ship_id];
                let possible_distance = map.calculate_distance(&ship.position, &Position{x:possible_cells_list[0][0] as i32, y:possible_cells_list[0][1] as i32});
                if possible_distance < minimum_distance_to_dropoff {
                    minimum_distance_to_dropoff = possible_distance;
                    ship_id_for_dropoff = ship_id.clone();
                }
            }
            if !ship_status.contains_key(&ship_id_for_dropoff){
                ship_status.insert(ship_id_for_dropoff.clone(), ShipStates::SettlingDropoff); 
            }
            else {
                ship_status.remove(&ship_id_for_dropoff);
                ship_status.entry(ship_id_for_dropoff).or_insert(ShipStates::SettlingDropoff);
            }
            dropoff_creating = 1;
        }

        
        for ship_id in &me.ship_ids {
            let ship = &game.ships[ship_id];
            let cell = map.at_entity(ship);

            
            if !ship_status.contains_key(ship_id){
                ship_status.insert(ship_id.clone(), ShipStates::Exploring); 
            }

            if dropoff_creating == 2 && dropoff_group_send > 1 && ship_status[ship_id] != ShipStates::GoTo && !ships_went_to_dropoff.contains(&ship_id.0) {

                Log::log(&format!("Ship {} will go to dropoff", ship_id.0));
                ships_went_to_dropoff.push(ship_id.clone().0);
                ship_status.remove(&ship_id);
                ship_status.entry(*ship_id).or_insert(ShipStates::GoTo);
                dropoff_group_send -= 1;
            }

            if &cell.position == &me.shipyard.position {
                ship_status.remove(&ship_id);
                ship_status.entry(*ship_id).or_insert(ShipStates::Exploring);
            }
            for dropoff in &me.dropoff_ids{
                if &cell.position == &game.dropoffs[dropoff].position {
                    ship_status.remove(&ship_id);
                    ship_status.entry(*ship_id).or_insert(ShipStates::Exploring);
                }
            }
            if ship_status[ship_id] != ShipStates::Returning && ship_status[ship_id] != ShipStates::SettlingDropoff && ship_status[ship_id] != ShipStates::GoTo && ship.halite >= ((game.constants.max_halite as f32) * return_minimum) as usize{
                Log::log(&format!("Ship {} have more than 980 halite, returning", ship_id.0));
                ship_status.remove(&ship_id);
                ship_status.entry(*ship_id).or_insert(ShipStates::Returning);
            }
            if ship_status[ship_id] == ShipStates::GoTo{
                let mut closest_distance = 99999;
                let mut closest_id = hlt::DropoffId(0);
                for dropoff_id in &me.dropoff_ids{
                    let distance_to_dropoff = map.calculate_distance(&ship.position, &game.dropoffs[dropoff_id].position);
                    if distance_to_dropoff <= closest_distance{
                        closest_distance = distance_to_dropoff.clone();
                        closest_id = *dropoff_id;
                    }
                }
                if closest_distance < 5 {
                    Log::log(&format!("Ship {} already got close to dropoff {}", ship_id.0, closest_id.0));
                    ship_status.remove(&ship_id);
                    ship_status.entry(*ship_id).or_insert(ShipStates::Exploring);
                }
            }
            if ship_status[ship_id] == ShipStates::Mining && cell.halite <= 20 {
                ship_status.remove(&ship_id);
                ship_status.entry(*ship_id).or_insert(ShipStates::Exploring);
            }
            if ship_status[ship_id] != ShipStates::Returning && ship_status[ship_id] != ShipStates::SettlingDropoff && ship_status[ship_id] != ShipStates::GoTo && cell.halite > (game.constants.max_halite / divider) as usize {
                ship_status.remove(&ship_id);
                ship_status.entry(*ship_id).or_insert(ShipStates::Mining);
            }
            let command = match ship_status[ship_id] {
                ShipStates::Returning => {
                    let mut closest_distance = 99999;
                    let mut closest_id = hlt::DropoffId(0);
                    for dropoff_id in &me.dropoff_ids{
                        let distance_to_dropoff = map.calculate_distance(&ship.position, &game.dropoffs[dropoff_id].position);
                        if distance_to_dropoff <= closest_distance{
                            closest_distance = distance_to_dropoff;
                            closest_id = *dropoff_id;
                        }
                    }
                    let distance_to_shipyard = map.calculate_distance(&ship.position, &me.shipyard.position);
                    let direction = if distance_to_shipyard < closest_distance {
                        navi.naive_navigate(&ship, &me.shipyard.position)
                    }
                    else {
                        navi.naive_navigate(&ship, &game.dropoffs[&closest_id].position)
                    };
                    ship.move_ship(direction)
                },
                ShipStates::Exploring => {
                    let random_direction = Direction::get_all_cardinals();
                    let mut max_halite_dir = Direction::Still;
                    let mut max_halite = (cell.halite as f32 * exploring_move_multiplier) as usize;
                    let mut safe_moves = Vec::new();
                    for possible_direction in random_direction {
                        let target_pos = &ship.position.directional_offset(possible_direction);
                        if navi.is_safe(&target_pos){
                            safe_moves.push(possible_direction);
                            if map.at_position(target_pos).halite > max_halite {
                                max_halite = map.at_position(target_pos).halite;
                                max_halite_dir = possible_direction;
                            }
                        }
                    }
                    let mut closest_distance = 99999;
                    let mut closest_id = hlt::DropoffId(0);
                    for dropoff_id in &me.dropoff_ids{
                        let distance_to_dropoff = map.calculate_distance(&ship.position, &game.dropoffs[dropoff_id].position);
                        if distance_to_dropoff <= closest_distance{
                            closest_distance = distance_to_dropoff;
                            closest_id = *dropoff_id;
                        }
                    }
                    let distance_to_shipyard = map.calculate_distance(&ship.position, &me.shipyard.position);
                    if (distance_to_shipyard < 2 || closest_distance < 2) && ((safe_moves.len() < 4 && safe_moves.len() > 1) || max_halite == 0) {
                        let target_pos = if safe_moves.len() > 1 {
                            ship.position.directional_offset(safe_moves[rng.gen_range(0, safe_moves.len())])
                        }
                        else if safe_moves.len() == 1 {
                            ship.position.directional_offset(safe_moves[0])
                        }
                        else {
                            ship.position.directional_offset(max_halite_dir)
                        };
                        let direction = navi.naive_navigate(&ship, &target_pos); //max_halite_dir)
                        ship.move_ship(direction)
                    }
                    else {
                        let target_pos = ship.position.directional_offset(max_halite_dir);
                        let direction = navi.naive_navigate(&ship, &target_pos); //max_halite_dir)
                        ship.move_ship(direction)
                    }
                },
                ShipStates::Mining => {
                    ship.stay_still()
                },
                ShipStates::SettlingDropoff => {
                    let direction = navi.naive_navigate(&ship, &Position{x: possible_cells_list[0][0] as i32, y: possible_cells_list[0][1] as i32});
                    let cell_free = &cell.structure.is_some();
                    if direction == Direction::Still && (me.halite + &cell.halite + ship.halite) > 5000 && !cell_free.clone() {
                        dropoff_creating = 2;
                        ship.make_dropoff()
                    } else if cell_free.clone() && direction == Direction::Still {
                        possible_cells_list[0][0] += 1;
                        ship.move_ship(direction)
                    } else {
                        ship.move_ship(direction)
                    }
                },
                ShipStates::GoTo => {
                        let mut closest_distance = 99999;
                        let mut closest_id = hlt::DropoffId(999);
                        for dropoff_id in &me.dropoff_ids{
                            let distance_to_dropoff = map.calculate_distance(&ship.position, &game.dropoffs[dropoff_id].position);
                            if distance_to_dropoff <= closest_distance{
                                closest_distance = distance_to_dropoff;
                                closest_id = *dropoff_id;
                            }
                        }
                        if closest_distance < 99999{
                            let direction = navi.naive_navigate(&ship, &game.dropoffs[&closest_id].position);
                            ship.move_ship(direction)
                        } 
                        else {
                            ship.stay_still()
                        }
                },
                _ => {
                    ship.stay_still()
                }
            };
            command_queue.push(command);
        }

        
        if (game_length - game.turn_number) <= random_death_turn { //destroy ships

            navi = backup_navi.clone();
            Log::log("Random death!");
            command_queue = Vec::<Command>::new(); //Overide previous rules
            for ship_id in &me.ship_ids {
                let ship = &game.ships[ship_id];
                let cell = map.at_entity(ship);
                if !navi.is_safe(&me.shipyard.position) {
                    navi.mark_safe(&me.shipyard.position)
                }
                let mut closest_distance = 99999;
                let mut closest_id = hlt::DropoffId(0);
                for dropoff_id in &me.dropoff_ids{
                    let distance_to_dropoff = map.calculate_distance(&ship.position, &game.dropoffs[dropoff_id].position);
                    if !navi.is_safe(&game.dropoffs[dropoff_id].position) {
                        navi.mark_safe(&game.dropoffs[dropoff_id].position);
                    }
                    if distance_to_dropoff <= closest_distance{
                        closest_distance = distance_to_dropoff;
                        closest_id = *dropoff_id;
                    }
                }
                let distance_to_shipyard = map.calculate_distance(&ship.position, &me.shipyard.position);
                let direction = if distance_to_shipyard < closest_distance {
                    navi.naive_navigate(&ship, &me.shipyard.position)
                }
                else {
                    navi.naive_navigate(&ship, &game.dropoffs[&closest_id].position)
                };
                let target_pos = ship.position.directional_offset(direction);
                command_queue.push(ship.move_ship(direction));
            }
            //Log::log(&format!("Steps: {:?}", &command_queue));

        }

        if
            (game_length - game.turn_number) >= stop_producing_ships_turn &&
            me.halite >= game.constants.ship_cost &&
            navi.is_safe(&me.shipyard.position) &&
            dropoff_creating != 1 &&
            ((dropoff_turn - reserve_dropoff_halite) >= game.turn_number ||
            dropoff_turn < game.turn_number)
            
        {
            command_queue.push(me.shipyard.spawn());
        }


        Game::end_turn(&command_queue);
    }
}
