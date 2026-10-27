--!strict
-- The test content's sprites' timing, by name: made up, short, as the tests reason about it (nothing from a
-- ROM). Each animation is its frames' durations in ticks, `loop = true` where it loops. The test content's own
-- sprites (sprites.luau) are timed so, and so are EXE6's that its EXE6 modules show (nettai_battle's
-- content::testing defines each in place of EXE6's module for it); one this hasn't has no animation, and holds a
-- single frame.
return {
    ["aqua-surge"] = { { 6, loop = true }, { 6, loop = true }, { 6, loop = true } },
    ["bass-anly"] = {
        { 8, 8, loop = true }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 },
        { 4, 4, loop = true }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 },
        { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 },
    },
    ["beast-shot"] = { { 2, 2, loop = true } },
    ["boulder"] = { { 30, loop = true } },
    ["bubble"] = { { 4 }, { 4, 4, loop = true } },
    ["buster-arm"] = {
        { 30, loop = true }, { 30, loop = true }, { 30, loop = true }, { 30, loop = true }, { 30, loop = true },
        { 30, loop = true }, { 30, loop = true }, { 30, loop = true }, { 30, loop = true }, { 30, loop = true },
        { 30, loop = true }, { 30, loop = true }, { 30, loop = true }, { 30, loop = true }, { 30, loop = true },
        { 30, loop = true }, { 30, loop = true }, { 30, loop = true }, { 30, loop = true }, { 30, loop = true },
        { 30, loop = true }, { 30, loop = true }, { 30, loop = true }, { 30, loop = true }, { 30, loop = true },
    },
    ["count"] = {
        { 8, 8, loop = true }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 },
        { 3, 3, loop = true },
    },
    ["countdown-bomb"] = { { 3, 3 }, { 20, loop = true }, { 3, 3 }, { 20, loop = true } },
    ["crack-shot"] = { { 2, 2, loop = true } },
    ["django"] = {
        { 8, 8, loop = true }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4, 4, loop = true }, { 4, 4, loop = true },
        { 4 }, { 4 },
    },
    ["dragon"] = {
        { 6, loop = true }, { 6, loop = true }, { 6, loop = true }, { 6, loop = true }, { 6, loop = true },
        { 6, loop = true }, { 6, loop = true }, { 6, loop = true },
    },
    ["dust-storm-mote"] = { { 3, 3, loop = true } },
    ["dustman"] = {
        { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 },
        { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4, 4, loop = true }, { 10, 20 },
    },
    ["element-pillar-flames"] = { { 4, loop = true }, { 2, 2 } },
    ["element-pillar-lightning"] = { { 4, loop = true }, { 2, 2 } },
    ["elmnt-ice"] = { { 4 }, { 4, 4, loop = true } },
    ["elmnt-man"] = {
        { 8, 8, loop = true }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 },
        { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 },
    },
    ["erase-beam"] = { { 3, 3, loop = true }, { 3, 3, loop = true }, { 3, 3, loop = true } },
    ["erase-mark"] = { { 4, 4, loop = true } },
    ["erase-ray"] = { { 3, 3 }, { 8, loop = true }, { 4 } },
    ["eraseman"] = {
        { 8, 8, loop = true }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 },
        { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4, 40 },
    },
    ["explosion"] = { { 3, 3, 3 } },
    ["falling-rock"] = { { 8, loop = true }, { 4, loop = true } },
    ["grab-shot"] = { { 8, loop = true }, { 3, 3 } },
    ["gun-del-sol"] = { { 3, 3 }, { 2, 2, loop = true }, { 4 } },
    ["gust"] = { { 4, 4, loop = true } },
    ["hive"] = { { 30, loop = true }, { 4, 30 } },
    ["honey-bee"] = { { 2, 2, loop = true } },
    ["impact"] = { { 2, 2 } },
    ["land-mine"] = { { 4, 4, loop = true } },
    ["lightning"] = { { 3, 3, loop = true } },
    ["meteor"] = { { 2, 2, loop = true } },
    ["moon-beam"] = {
        { 8, 8, loop = true }, { 8, 8, loop = true }, { 8, 8, loop = true }, { 8, 8, loop = true },
        { 8, 8, loop = true },
    },
    ["muzzle-flash"] = { { 2, 2 } },
    ["otenko"] = { { 2, 2, 2 }, { 8, 8, 8, 8, loop = true } },
    ["panel-strike"] = { { 3, 3 } },
    ["pink-flash"] = { { 2, 3 } },
    ["reflector-shield"] = { { 8, loop = true }, { 7, 7 } },
    ["rock-cubes"] = { { 3, 3, 3 }, { 30, loop = true }, { 30, loop = true } },
    ["rock-debris"] = { { 6 }, { 6 }, { 6 }, { 6 } },
    ["small-puff"] = { { 4 }, { 4, 4, 4 } },
    ["spout-geyser"] = { { 3, 3, loop = true }, { 3, 3, loop = true }, { 3, 3, loop = true } },
    ["spout-pillar"] = {
        { 3, 3, loop = true }, { 3, 3, loop = true }, { 3, 3, loop = true }, { 3, 3, loop = true },
    },
    ["spout-splash"] = { { 5, 5, loop = true } },
    ["spoutman"] = {
        { 8, 8, loop = true }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 },
        { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 },
    },
    ["sprite-14-02"] = { { 4 }, { 4, 4, 4 } },
    ["sun-beam"] = { { 2, 2, loop = true } },
    ["sun-beam-ex"] = { { 2, 2, loop = true } },
    ["sword-slash"] = { { 3, 3 } },
    ["test-burst"] = { { 3, 3, 3 } },
    ["test-navi"] = {
        { 16, loop = true }, { 4 }, { 4 }, { 2, 3 }, { 1, 2 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 },
        { 6, 30, loop = true }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 },
        { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 }, { 4 },
    },
    ["test-spark"] = { { 2, 2 } },
    ["test-sprite-bubble"] = { { 4 }, { 4, 4, loop = true } },
    ["test-sprite-idle-overlay"] = { { 6, 6, loop = true } },
    ["test-sprite-immobilized"] = { { 3, 3, 3 }, { 30, loop = true }, { 30, loop = true } },
    ["whirlwind"] = { { 3, 3, loop = true } },
}
