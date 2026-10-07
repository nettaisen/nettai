--!strict
-- What an EXE5 link battle picks at random with its settings (0x08129F2C, EXE6's `sub_81209DC`: `PosRNG1()` for the
-- stage, then `PosRNG2()` for the background). Whoever makes a random match picks from these (the engine picks none:
-- a round's settings state its stage and background); a match may name any link battle stage and any background all
-- the same.

local stages = require("../stages")

-- The settings' effects a link battle stage has (the link battle's) and hasn't (the random battle's).
local LINK, RANDOM = 0x8, 0x200000

-- The stages a match may name: the game's link battle stages, each a settings record with the link effect that isn't the random battle's, whether a random pick reaches them or not.
local function match_stages(): { StageDef }
    local out: { StageDef } = {}
    for _, s in stages :: { [string]: StageDef } do
        local effects = s.effects or 0
        if bit32.band(effects, LINK) ~= 0 and bit32.band(effects, RANDOM) == 0 then
            table.insert(out, s)
        end
    end
    return out
end

return {
    -- The stages a link battle picks one of at random with its settings, by the pick's index (0x08129F2C, EXE6's
    -- `sub_81209DC`): `PosRNG1() % 0x60` (the counts by mode at 0x08129F64: 0x44 and 0x60) into the settings list at
    -- 0x0811AF4C, and then an index from 76 to 87 has 76 taken off. So the list is the original's as it stands, and
    -- nothing to tidy: the first twelve records' stages are there twice (indices 0 to 11 and 76 to 87), the stages of
    -- records 76 to 87 (netbattle-75 to netbattle-86) never, and netbattle-51 and netbattle-52 twice because records
    -- 50 and 52, and 51 and 53, are the same. Whoever makes a random match picks from them; a match may still name
    -- any link battle stage.
    stages = {
        stages["netbattle-1"], stages["netbattle-2"], stages["netbattle-3"], stages["netbattle-4"], stages["netbattle-5"], stages["netbattle-6"],
        stages["netbattle-7"], stages["netbattle-8"], stages["netbattle-9"], stages["netbattle-10"], stages["netbattle-11"], stages["netbattle-12"],
        stages["netbattle-13"], stages["netbattle-14"], stages["netbattle-15"], stages["netbattle-16"], stages["netbattle-17"], stages["netbattle-18"],
        stages["netbattle-19"], stages["netbattle-20"], stages["netbattle-21"], stages["netbattle-22"], stages["netbattle-23"], stages["netbattle-24"],
        stages["netbattle-25"], stages["netbattle-26"], stages["netbattle-27"], stages["netbattle-28"], stages["netbattle-29"], stages["netbattle-30"],
        stages["netbattle-31"], stages["netbattle-32"], stages["netbattle-33"], stages["netbattle-34"], stages["netbattle-35"], stages["netbattle-36"],
        stages["netbattle-37"], stages["netbattle-38"], stages["netbattle-39"], stages["netbattle-40"], stages["netbattle-41"], stages["netbattle-42"],
        stages["netbattle-43"], stages["netbattle-44"], stages["netbattle-45"], stages["netbattle-46"], stages["netbattle-47"], stages["netbattle-48"],
        stages["netbattle-49"], stages["netbattle-50"], stages["netbattle-51"], stages["netbattle-52"], stages["netbattle-51"], stages["netbattle-52"],
        stages["netbattle-53"], stages["netbattle-54"], stages["netbattle-55"], stages["netbattle-56"], stages["netbattle-57"], stages["netbattle-58"],
        stages["netbattle-59"], stages["netbattle-60"], stages["netbattle-61"], stages["netbattle-62"], stages["netbattle-63"], stages["netbattle-64"],
        stages["netbattle-65"], stages["netbattle-66"], stages["netbattle-67"], stages["netbattle-68"], stages["netbattle-69"], stages["netbattle-70"],
        stages["netbattle-71"], stages["netbattle-72"], stages["netbattle-73"], stages["netbattle-74"], stages["netbattle-1"], stages["netbattle-2"],
        stages["netbattle-3"], stages["netbattle-4"], stages["netbattle-5"], stages["netbattle-6"], stages["netbattle-7"], stages["netbattle-8"],
        stages["netbattle-9"], stages["netbattle-10"], stages["netbattle-11"], stages["netbattle-12"], stages["netbattle-87"], stages["netbattle-88"],
        stages["netbattle-89"], stages["netbattle-90"], stages["netbattle-91"], stages["netbattle-92"], stages["netbattle-93"], stages["netbattle-94"],
    },
    -- How many of them, from the first, a set's first round picks among: the count the comm menu passes for the first
    -- round goes by practice or not (0x08134F20: the practice row's 0, the battle row's 1, whatever the single or
    -- triple battle), and a random match is a triple battle's practice (nettai-match's effects, 0x0813614C's 0x600):
    -- 0x44, the first 68 (netbattle-1 to netbattle-66, 51 and 52 twice). The fold, from index 76, is past them; the
    -- rounds after pick among all 96 (0x08009970's count 0x60).
    first_round_stages = 68,
    -- The stages a match may name (a match file's, the editor's, an offer's): the link battle's, those the pick never
    -- reaches (netbattle-75 to netbattle-86) too.
    match_stages = match_stages(),
    -- The backgrounds a link battle picks one of at random with its settings (`table[PosRNG2() % 27]`, the table at
    -- 0x08129F6C: its backgrounds 0x00 to 0x1A, each once, in the table's order). A liberation's map's number is its
    -- area's background (the same load data: compat/assets.toml), so acdc-area, oran-area, scilab-area and undernet
    -- are here twice and end-area three times, as the table's numbers are. A stage's own background (every record's
    -- is the same one) is what a match that names none shows.
    backgrounds = {
        asset.background("main-comp"), asset.background("drill-comp"), asset.background("lans-hp"),
        asset.background("mayls-hp"), asset.background("dexs-hp"), asset.background("yais-hp"),
        asset.background("comp"), asset.background("comp-alt"), asset.background("acdc-area"),
        asset.background("oran-area"), asset.background("scilab-area"), asset.background("visionburst"),
        asset.background("acdc-area"), asset.background("oran-area"), asset.background("scilab-area"),
        asset.background("end-area"), asset.background("ship-comp"), asset.background("scilab-hp"),
        asset.background("end-area"), asset.background("gargcastle-hp"), asset.background("end-area"),
        asset.background("soulserv-comp"), asset.background("gargoyle-comp"), asset.background("factory-comp"),
        asset.background("undernet"), asset.background("nebula-area"), asset.background("undernet"),
    },
}
