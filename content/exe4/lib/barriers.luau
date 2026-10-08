--!strict
-- EXE4's barriers (FstBarr's, the patch cards' auras at the start, the barrier chips'): by EXE4's barrier types, the
-- rows of 0x080185E0 (HP, threshold, timer) and of its visual's looks, 0x080E23D8 (sprite, animation, palette, the
-- parts MegaMan's hides), raised by 0x08012DCE with EXE4's visual (../objects/barrier_visual: effect object 7). The
-- barrier's tick is the rules' (`status.intake.barrier`). Types 4 (150 HP, regrowing 180 ticks after it breaks),
-- 5 (1 HP) and 7 (a threshold of 300 for 3000 ticks) wait on the chips that raise them (docs/design/exe4-map.md §18
-- item 53). The visual sits on its navi by EXE4's own offsets, not an attach point: a look's `attach_point` is
-- unread.
--
-- The sheets are EXE6's numbers (0c-3d, 0c-07).

local barrier_visual = require("../objects/barrier_visual")
local common_visual = require("@exelib/barriers/visual")

local barriers = {}

local SPRITE, AURA = asset.sprite("barrier"), asset.sprite("aura")
-- The visual's rising sound (0x080E2496), going back up too (0x080E25E4).
local SOUND = asset.sound("barrier")
local NO_TIMER = 0xFFFF
-- The parts MegaMan's aura hides (0x080E23D8's word for types 6 and 7: part 7).
local PART_7 = 0x1000000

local function barrier(b: Barrier): Barrier
    return new.record("barrier", b)
end

-- Type 1: 10 HP (FstBarr's, the Barrier chip's).
barriers.barrier_10 = barrier {
    id = "barrier/10",
    behavior = "plain",
    hp = 10,
    threshold = 0,
    timer = NO_TIMER,
    look = { sprite = SPRITE, anim = 0, palette = 0, attach_point = 0, hidden_parts = 0 },
    sound = SOUND,
}

-- Type 2: 100 HP.
barriers.barrier_100 = barrier {
    id = "barrier/100",
    behavior = "plain",
    hp = 100,
    threshold = 0,
    timer = NO_TIMER,
    look = { sprite = SPRITE, anim = 0, palette = 3, attach_point = 0, hidden_parts = 0 },
    sound = SOUND,
}

-- Type 3: 200 HP.
barriers.barrier_200 = barrier {
    id = "barrier/200",
    behavior = "plain",
    hp = 200,
    threshold = 0,
    timer = NO_TIMER,
    look = { sprite = SPRITE, anim = 0, palette = 6, attach_point = 0, hidden_parts = 0 },
    sound = SOUND,
}

-- Type 6: an aura that stops every hit of less than 200 for 3000 ticks.
barriers.life_aura = barrier {
    id = "barrier/life-aura",
    behavior = "plain",
    hp = 1,
    threshold = 200,
    timer = 3000,
    look = { sprite = AURA, anim = 0, palette = 0, attach_point = 0, hidden_parts = PART_7 },
    sound = SOUND,
}

-- 0x08012DCE and the visual (0x080E2622): `b` on `navi`; its old visual, if it has one, ends first (0x080E264A) unless
-- `keep_old` (FstBarr's, at the navi's init, which reads none: the navi has no visual yet); a new one comes.
function barriers.raise(navi: Object, b: Barrier, keep_old: boolean?)
    common_visual.raise_barrier(navi, b)
    local old = navi.barrier_visual
    if old and not keep_old then
        common_visual.finish(old)
    end
    barrier_visual.spawn(navi, b)
end

return barriers
