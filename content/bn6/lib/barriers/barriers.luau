--!strict
-- Barriers (docs/engine/dimming-chips.md §3): a navi's barrier is a row of
-- the original's `byte_8020B2C` and `byte_8020B8C` (`sub_801A7CC` sets it
-- in the collision data; the ruleset's `sub_801A802` wears it down) with
-- its visual's look (`byte_80E0A14`, visual.luau). The barriers the game
-- raises in a netbattle, by type: 1 the Barrier chip's and FirstBarrier's,
-- 5 Barr100's and Tango's heal's, 7 Barr200's, 8 BblWrap's bubble, 9
-- LifeAur's aura. (The other types, 0, 2-4, 6 and 10-15, are only a navi
-- AI's.)

local visual = require("./visual")

local barriers = {}

-- A barrier.
function barriers.barrier(b: Barrier): Barrier
    return define.record("barrier", b)
end

local SPRITE, BUBBLE, AURA = asset.sprite("barrier"), asset.sprite("bubble"), asset.sprite("aura")
local SOUND, BUBBLE_SOUND, BUBBLE_POP = asset.sound("barrier"), asset.sound("bubble"), asset.sound("bubble-pop")
local NO_TIMER = 0xFFFF

-- Type 1: 10 HP.
barriers.barrier_10 = barriers.barrier {
    behavior = "plain",
    hp = 10,
    threshold = 0,
    timer = NO_TIMER,
    look = { sprite = SPRITE, anim = 0, palette = 0, attach_point = 2, hidden_parts = 0 },
    sound = SOUND,
}

-- Type 5: 100 HP.
barriers.barrier_100 = barriers.barrier {
    behavior = "plain",
    hp = 100,
    threshold = 0,
    timer = NO_TIMER,
    look = { sprite = SPRITE, anim = 0, palette = 3, attach_point = 2, hidden_parts = 0 },
    sound = SOUND,
}

-- Type 7: 200 HP.
barriers.barrier_200 = barriers.barrier {
    behavior = "plain",
    hp = 200,
    threshold = 0,
    timer = NO_TIMER,
    look = { sprite = SPRITE, anim = 0, palette = 6, attach_point = 2, hidden_parts = 0 },
    sound = SOUND,
}

-- Type 8: a bubble that takes one hit, grows back 240 ticks later, and
-- breaks on elec.
barriers.bubble = barriers.barrier {
    behavior = "bubble",
    hp = 1,
    threshold = 0,
    timer = NO_TIMER,
    look = { sprite = BUBBLE, anim = 0, palette = 0, attach_point = 0x15, hidden_parts = 0 },
    sound = BUBBLE_SOUND,
    down_sound = BUBBLE_POP,
}

-- Type 9: an aura that stops every hit of less than 200 for 3000 ticks.
barriers.life_aura = barriers.barrier {
    behavior = "plain",
    hp = 1,
    threshold = 200,
    timer = 3000,
    look = { sprite = AURA, anim = 0, palette = 0, attach_point = 1, hidden_parts = 0 },
    hidden_unless_local_megaman = true,
    sound = SOUND,
}

-- `sub_801A7CC` and the visual: `b` on `navi`; its old visual, if it has
-- one, ends first (`sub_80E0DC0`) unless `keep_old` (FirstBarrier's: the
-- navi has none yet); a new one comes (`sub_80E0D98`).
function barriers.raise(navi: Object, b: Barrier, keep_old: boolean?)
    visual.raise_barrier(navi, b)
    local old = navi.barrier_visual
    if old and not keep_old then
        visual.finish(old)
    end
    visual.spawn(navi, b)
end

return barriers
