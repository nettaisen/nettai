--!strict
-- What an EXE6 link battle picks at random with its settings (`sub_81209DC`, twice a round for the two rounds after:
-- `PosRNG1()` for the stage, then `PosRNG2()` for the background). Whoever makes a random match picks from these
-- (the engine picks none: a round's settings state its stage and background); a match may name any link battle stage
-- and any background all the same.

local stages = require("../stages")

return {
    -- The stages a link battle picks one of at random with its settings, by the pick's index (`sub_81209DC`:
    -- `PosRNG1() % 0x60` into `BattleSettingsList1`, its first 96 records, each a stage of its own). Whoever makes a
    -- random match picks from them; a match may still name any link battle stage.
    stages = {
        stages.netbattle_1, stages.netbattle_2, stages.netbattle_3, stages.netbattle_4, stages.netbattle_5, stages.netbattle_6,
        stages.netbattle_7, stages.netbattle_8, stages.netbattle_9, stages.netbattle_10, stages.netbattle_11, stages.netbattle_12,
        stages.netbattle_13, stages.netbattle_14, stages.netbattle_15, stages.netbattle_16, stages.netbattle_17, stages.netbattle_18,
        stages.netbattle_19, stages.netbattle_20, stages.netbattle_21, stages.netbattle_22, stages.netbattle_23, stages.netbattle_24,
        stages.netbattle_25, stages.netbattle_26, stages.netbattle_27, stages.netbattle_28, stages.netbattle_29, stages.netbattle_30,
        stages.netbattle_31, stages.netbattle_32, stages.netbattle_33, stages.netbattle_34, stages.netbattle_35, stages.netbattle_36,
        stages.netbattle_37, stages.netbattle_38, stages.netbattle_39, stages.netbattle_40, stages.netbattle_41, stages.netbattle_42,
        stages.netbattle_43, stages.netbattle_44, stages.netbattle_45, stages.netbattle_46, stages.netbattle_47, stages.netbattle_48,
        stages.netbattle_49, stages.netbattle_50, stages.netbattle_51, stages.netbattle_52, stages.netbattle_53, stages.netbattle_54,
        stages.netbattle_55, stages.netbattle_56, stages.netbattle_57, stages.netbattle_58, stages.netbattle_59, stages.netbattle_60,
        stages.netbattle_61, stages.netbattle_62, stages.netbattle_63, stages.netbattle_64, stages.netbattle_65, stages.netbattle_66,
        stages.netbattle_67, stages.netbattle_68, stages.netbattle_69, stages.netbattle_70, stages.netbattle_71, stages.netbattle_72,
        stages.netbattle_73, stages.netbattle_74, stages.netbattle_75, stages.netbattle_76, stages.netbattle_77, stages.netbattle_78,
        stages.netbattle_79, stages.netbattle_80, stages.netbattle_81, stages.netbattle_82, stages.netbattle_83, stages.netbattle_84,
        stages.netbattle_85, stages.netbattle_86, stages.netbattle_87, stages.netbattle_88, stages.netbattle_89, stages.netbattle_90,
        stages.netbattle_91, stages.netbattle_92, stages.netbattle_93, stages.netbattle_94, stages.netbattle_95, stages.netbattle_96,
    },
    -- How many of them, from the first, a set's first round picks among: the count the comm menu passes for the first
    -- round goes by the match type (`sub_812A76C`: a single battle's mode 0, a triple battle's mode 1, a random
    -- battle's mode 2, `dword_8120A14`'s 0x44, 0x60 and 0x44 past the records' 0x60), and a random match is a triple
    -- battle's practice (nettai-match's effects, `off_812B7AC`'s 0x600): 0x60, all of them, as the rounds after.
    first_round_stages = 96,
    -- The backgrounds a link battle picks one of at random with its settings (`sub_81209DC`: `byte_8120A20`, 21
    -- entries, in the table's order; three are there twice, so twice as likely). Whoever makes a random match picks
    -- from them (the engine picks none: a round's settings state its background).
    backgrounds = {
        asset.background("lans-hp"), asset.background("acdc-hp"), asset.background("acdc-hp"),
        asset.background("aquarium-hp"), asset.background("sky-hp"), asset.background("green-hp"),
        asset.background("robot-control-comp"), asset.background("comp"), asset.background("comp-alt"),
        asset.background("central-area"), asset.background("aquarium-comp"), asset.background("seaside-area"),
        asset.background("judgetree-comp"), asset.background("green-area"), asset.background("sky-area"),
        asset.background("undernet"), asset.background("mr-weather-comp"), asset.background("underground"),
        asset.background("underground"), asset.background("acdc-area"), asset.background("acdc-area"),
    },
}
