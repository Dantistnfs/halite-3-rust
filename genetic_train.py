import random 
import os
import subprocess
import json
from trueskill import Rating, quality_1vs1, rate_1vs1
from tqdm import tqdm

sizes = [32, 40, 48, 56, 64]


bot_values_dict = {
        "filter_divider": 6,
        "dropoff_turn": 120,
        "reserve_dropoff_halite": 30,
        "dropoff_distance_penalty" :100,
        "dropoff_group_send" : 15,
        "search_radius" : 7,
        "random_death_turn": 20,
        "exploring_move_multiplier": 125
        }




def gen_player(parameters):
    base_string = "\"RUST_BACKTRACE=1 ./target/release/my_bot"
    for key, value in parameters.items():
        base_string += " " + str(value)
    base_string += "\""
    
    return [base_string, Rating(), parameters]



def gen_match(size, bots_list):
    string = "./halite --no-timeout --no-replay --no-logs -v --results-as-json --width {0} --height {0} ".format(size)
    for bot in bots_list:
        string += bot[0] + " "
    return string

generation = 0

while True:
    print("Generation: ", generation)
    #generate variants
    variants = []
    for i in range(0,10):
        temp_dict = bot_values_dict.copy()
        for key, value in temp_dict.items():
            if random.random() > 0.7:
                temp_dict[key] = int(value * (1 + random.normalvariate(0,8)/100))
        variants.append(temp_dict.copy())


    global_bots_list = []
    for variant in variants:
        global_bots_list.append(gen_player(variant))


    for i in tqdm(range(0,1000)):
        bots_num = random.sample(range(0,len(global_bots_list)), 2)

        bots = [global_bots_list[bots_num[0]], global_bots_list[bots_num[1]]]
        command = gen_match(sizes[1], bots)
        b = subprocess.Popen(command, stdout=subprocess.PIPE, shell=True)
        out, err = b.communicate()
        bot_0_rank = json.loads(out.decode())['stats']["0"]["rank"]
        if bot_0_rank == 1: #so he won
            new_r1, new_r2 = rate_1vs1(bots[0][1], bots[1][1])
        else:
            new_r2, new_r1 = rate_1vs1(bots[1][1], bots[0][1])

        global_bots_list[bots_num[0]][1] = new_r1
        global_bots_list[bots_num[1]][1] = new_r2

    converted_list = []
    for bot in global_bots_list:
        item = [bot[2], (bot[1].mu - 3*bot[1].sigma), bot[1]]
        #print(item)
        converted_list.append(item)

    #print("Sorted list")
    sorted_list = sorted(converted_list, key=lambda x: x[1])
    print("Generation", generation, "winner:", sorted_list[-1])
    bot_values_dict = sorted_list[-1][0].copy()
    generation += 1
    #print(b)

    #if quality_1vs1(r1, r2) > 0.50:
    #new_r1, new_r2 = rate_1vs1(r1, r2)
    #print(b)
