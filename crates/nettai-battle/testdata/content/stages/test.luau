--!strict
-- The engine's test stages: two navis facing each other on plain panels
-- (in either order of placing), and the same with rock cubes or boulders
-- on the field (the EXE6 overlay's kinds).

local boulder = require("../objects/boulder")
local rock = require("../chips/rockcube/rock")

local PLAIN = {
    "normal normal normal normal normal normal",
    "normal normal normal normal normal normal",
    "normal normal normal normal normal normal",
}

local function stage(actors: { StageActor }): StageDef
    return {
        layout = PLAIN,
        panel_pattern = 0x38,
        music = asset.sound("test-stage-music"),
        background = asset.background("test-background"),
        mode = 0,
        battle_number = 0,
        -- A link battle.
        effects = 0x8,
        actors = actors,
    }
end

local function navi(side: number, x: number): StageActor
    return { place = "navi", side = side, x = x, y = 2 }
end

local function cube(x: number, y: number): StageActor
    return { place = rock.kind, variant = rock.variants.cube, x = x, y = y }
end

local function stone(x: number, y: number): StageActor
    return { place = boulder.kind, x = x, y = y }
end

local stages: { [string]: StageDef } = {
    ["test/link-battle"] = stage({ navi(1, 5), navi(0, 2) }),
    ["test/link-battle-side0-first"] = stage({ navi(0, 2), navi(1, 5) }),
    ["test/rock-battle"] = stage({ navi(0, 1), navi(1, 6), cube(3, 3), cube(4, 1) }),
    ["test/boulder-battle"] = stage({ navi(0, 1), navi(1, 6), stone(2, 2), stone(5, 2), stone(3, 1) }),
}

return stages
