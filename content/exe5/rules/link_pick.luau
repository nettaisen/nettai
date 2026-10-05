--!strict
-- What an EXE5 link battle picks at random with its settings (0x08129F2C, EXE6's `sub_81209DC`: `PosRNG1()` for the
-- stage, then `PosRNG2()` for the background). Whoever makes a random match picks from these (the engine picks none:
-- a round's settings state its stage and background); a match may name any link battle stage and any background all
-- the same.

local stages = require("../stages")

return {
    -- The stages a link battle picks one of at random with its settings, by the pick's index (0x08129F2C, EXE6's
    -- `sub_81209DC`): `PosRNG1() % 0x60` (the counts by mode at 0x08129F64: 0x44 and 0x60) into the settings list at
    -- 0x0811AF4C, and then an index from 76 to 87 has 76 taken off. So the list is the original's as it stands, and
    -- nothing to tidy: the first twelve records' stages are there twice (indices 0 to 11 and 76 to 87), the stages of
    -- records 76 to 87 (netbattle-75 to netbattle-86) never, and netbattle-51 and netbattle-52 twice because records
    -- 50 and 52, and 51 and 53, are the same. Whoever makes a random match picks from them; a match may still name
    -- any link battle stage.
    stages = {
        stages.netbattle_1, stages.netbattle_2, stages.netbattle_3, stages.netbattle_4, stages.netbattle_5, stages.netbattle_6,
        stages.netbattle_7, stages.netbattle_8, stages.netbattle_9, stages.netbattle_10, stages.netbattle_11, stages.netbattle_12,
        stages.netbattle_13, stages.netbattle_14, stages.netbattle_15, stages.netbattle_16, stages.netbattle_17, stages.netbattle_18,
        stages.netbattle_19, stages.netbattle_20, stages.netbattle_21, stages.netbattle_22, stages.netbattle_23, stages.netbattle_24,
        stages.netbattle_25, stages.netbattle_26, stages.netbattle_27, stages.netbattle_28, stages.netbattle_29, stages.netbattle_30,
        stages.netbattle_31, stages.netbattle_32, stages.netbattle_33, stages.netbattle_34, stages.netbattle_35, stages.netbattle_36,
        stages.netbattle_37, stages.netbattle_38, stages.netbattle_39, stages.netbattle_40, stages.netbattle_41, stages.netbattle_42,
        stages.netbattle_43, stages.netbattle_44, stages.netbattle_45, stages.netbattle_46, stages.netbattle_47, stages.netbattle_48,
        stages.netbattle_49, stages.netbattle_50, stages.netbattle_51, stages.netbattle_52, stages.netbattle_51, stages.netbattle_52,
        stages.netbattle_53, stages.netbattle_54, stages.netbattle_55, stages.netbattle_56, stages.netbattle_57, stages.netbattle_58,
        stages.netbattle_59, stages.netbattle_60, stages.netbattle_61, stages.netbattle_62, stages.netbattle_63, stages.netbattle_64,
        stages.netbattle_65, stages.netbattle_66, stages.netbattle_67, stages.netbattle_68, stages.netbattle_69, stages.netbattle_70,
        stages.netbattle_71, stages.netbattle_72, stages.netbattle_73, stages.netbattle_74, stages.netbattle_1, stages.netbattle_2,
        stages.netbattle_3, stages.netbattle_4, stages.netbattle_5, stages.netbattle_6, stages.netbattle_7, stages.netbattle_8,
        stages.netbattle_9, stages.netbattle_10, stages.netbattle_11, stages.netbattle_12, stages.netbattle_87, stages.netbattle_88,
        stages.netbattle_89, stages.netbattle_90, stages.netbattle_91, stages.netbattle_92, stages.netbattle_93, stages.netbattle_94,
    },
    -- The backgrounds a link battle picks one of at random with its settings (`table[PosRNG2() % 27]`, the table at
    -- 0x08129F6C: its backgrounds 0x00 to 0x1A, each once, in the table's order). A stage's own background (every
    -- record's is the same one) is what a match that names none shows.
    backgrounds = {
        asset.background("binary"), asset.background("drills"), asset.background("soccer-balls"),
        asset.background("rabbits-and-apples"), asset.background("faces-and-crosses"), asset.background("goldfish"),
        asset.background("calendar-squares"), asset.background("rings-blue"), asset.background("calendar-blue"),
        asset.background("calendar-gold"), asset.background("ones-and-zeros"), asset.background("grid"),
        asset.background("calendar-blue-2"), asset.background("calendar-gold-2"), asset.background("ones-and-zeros-2"),
        asset.background("numbers-3"), asset.background("ship-wheels"), asset.background("swirls"),
        asset.background("numbers"), asset.background("lions"), asset.background("numbers-2"),
        asset.background("crests"), asset.background("shuriken"), asset.background("microchips"),
        asset.background("glyphs-2"), asset.background("glyphs-blue"), asset.background("glyphs"),
    },
}
