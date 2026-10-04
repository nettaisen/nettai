--!strict
-- BN5's barriers (the barrier chips' and FstBarr's): BN6's rows (BN5's 0x0801CF94 and its visuals' looks 0x080E0A1C
-- are BN6's `byte_8020B2C` and `byte_80E0A14`, row for row; BN5 has no weak-element table, BN6's `byte_8020B8C`,
-- whose rows are all none), raised by BN5's code (0x080174DA) with BN5's visual (@exelib/barriers/visual: BN5's
-- effect object #7, 0x080E0AA0, which is never blown away and comes back up whatever its navi's status: BN5 has no
-- bubble status). The barrier chips' dimming controller is BN6's code (@exelib/barriers/controller: BN5's effect
-- object #0x2F, 0x080E3CCC).
--
-- The sheets are BN6's numbers (0c-3d, 0c-20, 0c-07: the same archives).

local common_controller = require("@exelib/barriers/controller")
local common_visual = require("@exelib/barriers/visual")

local barriers = {}

local visual = common_visual.make { id = "bn5:barrier-visual", blows = false }
barriers.visual = visual

local SPRITE, BUBBLE, AURA = asset.sprite("bn5:barrier"), asset.sprite("bn5:bubble"), asset.sprite("bn5:aura")
local SOUND, BUBBLE_SOUND, BUBBLE_POP = asset.sound("bn5:barrier"), asset.sound("bn5:bubble"), asset.sound("bn5:bubble-pop")
local NO_TIMER = 0xFFFF

local function barrier(b: Barrier): Barrier
    return define.record("barrier", b)
end

-- Type 1: 10 HP (the Barrier chip's and FstBarr's).
barriers.barrier_10 = barrier {
    id = "bn5:barrier/10",
    behavior = "plain",
    hp = 10,
    threshold = 0,
    timer = NO_TIMER,
    look = { sprite = SPRITE, anim = 0, palette = 0, attach_point = 2, hidden_parts = 0 },
    sound = SOUND,
}

-- Type 5: 100 HP.
barriers.barrier_100 = barrier {
    id = "bn5:barrier/100",
    behavior = "plain",
    hp = 100,
    threshold = 0,
    timer = NO_TIMER,
    look = { sprite = SPRITE, anim = 0, palette = 3, attach_point = 2, hidden_parts = 0 },
    sound = SOUND,
}

-- Type 7: 200 HP.
barriers.barrier_200 = barrier {
    id = "bn5:barrier/200",
    behavior = "plain",
    hp = 200,
    threshold = 0,
    timer = NO_TIMER,
    look = { sprite = SPRITE, anim = 0, palette = 6, attach_point = 2, hidden_parts = 0 },
    sound = SOUND,
}

-- Type 8: a bubble that takes one hit, grows back 240 ticks later, and breaks on elec.
barriers.bubble = barrier {
    id = "bn5:barrier/bubble",
    behavior = "bubble",
    hp = 1,
    threshold = 0,
    timer = NO_TIMER,
    look = { sprite = BUBBLE, anim = 0, palette = 0, attach_point = 0x15, hidden_parts = 0 },
    sound = BUBBLE_SOUND,
    down_sound = BUBBLE_POP,
}

-- Type 9: an aura that stops every hit of less than 200 for 3000 ticks.
barriers.life_aura = barrier {
    id = "bn5:barrier/life-aura",
    behavior = "plain",
    hp = 1,
    threshold = 200,
    timer = 3000,
    look = { sprite = AURA, anim = 0, palette = 0, attach_point = 1, hidden_parts = 0 },
    hidden_unless_local_megaman = true,
    sound = SOUND,
}

-- 0x080174DA and the visual: `b` on `navi`; its old visual, if it has one, ends first unless `keep_old` (FstBarr's:
-- the navi has none yet); a new one comes.
function barriers.raise(navi: Object, b: Barrier, keep_old: boolean?)
    common_visual.raise_barrier(navi, b)
    local old = navi.barrier_visual
    if old and not keep_old then
        common_visual.finish(old)
    end
    visual.spawn(navi, b)
end

barriers.controller = common_controller.make {
    id = "bn5:barriers/controller",
    raise = function(navi: Object, b: Barrier)
        barriers.raise(navi, b)
    end,
}

return barriers
