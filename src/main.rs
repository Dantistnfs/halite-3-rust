#[macro_use]
extern crate lazy_static;
extern crate rand;

use hlt::command::Command;
use hlt::direction::Direction;
use hlt::game::Game;
use hlt::log::Log;
use hlt::navi::Navi;
use rand::Rng;
use rand::SeedableRng;
use rand::XorShiftRng;
use std::env;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use std::collections::HashMap;

mod hlt;

#[derive(PartialEq, Debug)]
enum ShipStates {
Exploring,
Returning,
}



fn main() {
    let args: Vec<String> = env::args().collect();
    let rng_seed: u64 = if args.len() > 1 {
        args[1].parse().unwrap()
    } else {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    };
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
    Game::ready("MyRustBot");

    Log::log(&format!("Successfully created bot! My Player ID is {}. Bot rng seed is {}.", game.my_id.0, rng_seed));

    let game_length = match game.map.width {
        40 => 426,
        32 => 401,
        48 => 451,
        56 => 476,
        64 => 501,
        _ => 450
    };

    loop {
        game.update_frame();
        navi.update_frame(&game);
        let backup_navi = navi.clone();

        let me = &game.players[game.my_id.0];
        let map = &mut game.map;

        let mut command_queue: Vec<Command> = Vec::new();

        let divider = match game.turn_number {
            1..=20 => 2,
            21..=40 => 4,
            41..=60 => 5,
            61..=100 => 6,
            101..=200 => 7,
            _ => 20
        };

        let return_minimum = match game.turn_number {
            1..=100 => 0.98,
            101..=300 => 0.9,
            300..=400 => 0.8,
            _ => 0.7
        };
        
        for ship in &game.ships {
            if ship.1.position.x == me.shipyard.position.x && ship.1.position.y == me.shipyard.position.y && ship.1.owner != game.my_id {
                navi.mark_safe(&me.shipyard.position);
            }
        }



        for ship_id in &me.ship_ids {
            let ship = &game.ships[ship_id];
            let cell = map.at_entity(ship);

            
            if !ship_status.contains_key(ship_id){
                ship_status.insert(ship_id.clone(), ShipStates::Exploring); 
            }

            if ship_status[ship_id] == ShipStates::Returning {
                if &cell.position == &me.shipyard.position {
                    ship_status.remove(&ship_id);
                    ship_status.entry(*ship_id).or_insert(ShipStates::Exploring);
                }
                else {
                    let direction = navi.naive_navigate(&ship, &me.shipyard.position);
                    
                    command_queue.push(ship.move_ship(direction));
                    continue;
                }
            }
            else if ship.halite >= ((game.constants.max_halite as f32) * return_minimum) as usize{
                Log::log(&format!("Ship {} have more than 980 halite, returning", ship_id.0));
                ship_status.remove(&ship_id);
                ship_status.entry(*ship_id).or_insert(ShipStates::Returning);
            }

            let command = if cell.halite < game.constants.max_halite / divider {
                let random_direction = Direction::get_all_cardinals();
                let mut max_halite_dir = Direction::Still;
                let mut max_halite = 0;
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
                if ship.position == me.shipyard.position && safe_moves.len() == 1 {
                    let target_pos = &ship.position.directional_offset(safe_moves[0]);
                    let direction = navi.naive_navigate(&ship, &target_pos); //max_halite_dir)
                    ship.move_ship(direction)
                }
                else if max_halite <= ((cell.halite as f32) * 1.03) as usize{
                    navi.mark_unsafe(&ship.position, ship.id);
                    ship.stay_still()
                }
                else{
                    let target_pos = ship.position.directional_offset(max_halite_dir);
                    let direction = navi.naive_navigate(&ship, &target_pos); //max_halite_dir)
                    ship.move_ship(direction)
                }
            } else {
                navi.mark_unsafe(&ship.position, ship.id);
                ship.stay_still()

            };
            command_queue.push(command);
        }

        
        if (game_length - game.turn_number) <= 40 { //destroy ships

            navi = backup_navi.clone();
            Log::log("Random death!");
            command_queue = Vec::<Command>::new(); //Overide previous rules
            for ship_id in &me.ship_ids {
                let ship = &game.ships[ship_id];
                let cell = map.at_entity(ship);
                if !navi.is_safe(&me.shipyard.position) {
                    navi.mark_safe(&me.shipyard.position)
                }
                let direction = navi.naive_navigate(&ship, &me.shipyard.position);
                let target_pos = ship.position.directional_offset(direction);
                command_queue.push(ship.move_ship(direction));
            }
            //Log::log(&format!("Steps: {:?}", &command_queue));

        }

        if
            (game_length - game.turn_number) >= 200 &&
            me.halite >= game.constants.ship_cost &&
            navi.is_safe(&me.shipyard.position)
        {
            command_queue.push(me.shipyard.spawn());
        }


        Game::end_turn(&command_queue);
    }
}
