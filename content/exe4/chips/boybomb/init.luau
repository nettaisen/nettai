--!strict
-- BoyBomb1 to BoyBomb3: the dimming chips' variant 0x38, EXE5's BoyBomb with EXE4's numbers. A controller (effect
-- object #0x67, 0x080E94CC: the usual dimming phases, @exelib/dimming; its spawn 0x080E954A takes the user's side
-- alone) whose effect (0x080E9510) sets a bomb on one of the user's side's front panels at random (0x080E956C), with
-- the chip's damage and the telop's bonus, then waits 30 ticks. The bomb (attack object #0x3E, 0x080D4F40) is
-- @exelib/boybomb/bomb with EXE4's look.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local common = require("@exelib/boybomb/bomb")
local dimming_chips = require("@exelib/dimming")

type ControllerState = {
    -- The bonus the telop shows (the original's +0x32), added to the bomb's damage.
    bonus: number,
}

local WAIT = 30
-- The front panel the bomb takes: solid, with nothing on it, its owner bit as its side's.
local SOLID, OCCUPIED, SIDE_1 = 0x10, 0x0F88_0080, 0x20

-- 0x0801013A: back to its idle, no longer running while paused.
local function return_to_idle(me: Object)
    me.run_while_paused = false
    obstacle.return_to_idle(me)
end

-- The bomb (0x080D4F40): sprite 10-36; no NameID (0x080D4F72 writes none); a neutral body (row 0x0E) that everything
-- reaches (row 0x0F), its blast an attack at navis; EXE4's effects (0x080E1698): row 7 (0c-12) as it comes, row 0
-- (14-00) over the blast, row 0x0F (14-01) when it breaks, whatever ended it (0x080D51AE reads no removal); its sounds
-- (0xAA as it comes, 0xBC for the blast, 0x70 breaking); where it may stand by its side (0x080D53AC); a push keeps the
-- damage (0x0801393E); EXE4's obstacle rows (0x080D5068: row 3 does nothing, no frozen or bubbled ones); no panel
-- reserved, no tracking released; a bad panel or the battle's end breaks it even while dimmed (0x080D5024); its spawn
-- (0x080D51D6) takes the controller's side alone.
local BOMB = common.make {
    id = "boybomb/bomb",
    sprite = asset.sprite("boy-bomb"),
    body = { is = collision.neutral_body, hits = collision.everything },
    blast = {
        is = collision.attack,
        hits = collision.hits_navis,
        effect = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 },
        sound = asset.sound("heat-burst"),
    },
    comes = new.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0 },
    place_sound = asset.sound("place"),
    broken = new.effect { sprite = asset.sprite("impact"), anim = 0, palette = 0 },
    break_sound = asset.sound("hit-bomb-1"),
    stand_forbid = { [0] = 0x0B800000, [1] = 0x07800000 },
    push = "keeps_damage",
    actions = {
        [1] = return_to_idle,
        [3] = function(_me: Object) end,
        [4] = obstacle.paralyzed,
        [5] = obstacle.knocked_back,
    },
    breaks_while_dimmed = true,
    side_only = true,
}

-- 0x0800A244: in each row, walking from `side`'s far column (6 - 5 * `side`) back toward its own edge, `side`'s first
-- panel, kept when it meets (`need`, `forbid`).
local function front_panels(side: number, need: number, forbid: number): { { x: number, y: number } }
    local step = 2 * side - 1
    local found = {}
    for y = 1, 3 do
        local x = 6 - 5 * side
        while true do
            local p = field.panel(x, y)
            if not p then
                break
            end
            if p.alliance == side then
                if field.check(x, y, need, forbid) then
                    table.insert(found, { x = x, y = y })
                end
                break
            end
            x += step
        end
    end
    return found
end

-- 0x080E956C: one of the user's side's front panels at random, solid and free (0x080E95C0 by its side: the owner bit
-- 0x20 required for side 1's, forbidden for side 0's); without one, any of its front panels; none, nil.
local function pick(me: Object): (number?, number?)
    local side = me.alliance
    local need, forbid = SOLID, OCCUPIED
    if side == 1 then
        need = bit32.bor(need, SIDE_1)
    else
        forbid = bit32.bor(forbid, SIDE_1)
    end
    local found = front_panels(side, need, forbid)
    if #found == 0 then
        found = front_panels(side, 0, 0)
        if #found == 0 then
            return nil, nil
        end
    end
    local p = found[battle.rng_positive() % #found + 1]
    return p.x, p.y
end

-- 0x080E9510: the bomb on the picked panel, with the controller's damage word and bonus; then 30 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        local x, y = pick(me)
        if x and y then
            BOMB.spawn(me, x, y, me.element, dimming_chips.damage_plus(me, (me.state :: ControllerState).bonus))
        end
        me.timer = WAIT
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left == 0 then
        dimming_chips.done(me)
    end
end

local controller = new.kind {
    id = "boybomb/controller",
    pool = "effect",
    state = { bonus = "u16" },
    -- 0x080E94CC.
    update = dimming_chips.phases { name = "BoyBomb", effect = effect },
}

-- 0x080E954A: the chips' `dimming` hook, the controller with the dimming's bonus.
local function hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(controller, user, spec, true)
    if o then
        (o.state :: ControllerState).bonus = spec.bonus
    end
    return o
end

-- BoyBomb1 (EXE4 chip 0x055).
local boybomb1: Chip = {
    codes = { "E", "J", "M" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 0,
    mb = 30,
    damage = 220,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = hook,
}

-- BoyBomb2 (EXE4 chip 0x056).
local boybomb2: Chip = {
    codes = { "I", "S", "W" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 1,
    mb = 40,
    damage = 250,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = hook,
}

-- BoyBomb3 (EXE4 chip 0x057).
local boybomb3: Chip = {
    codes = { "G", "T", "V" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 280,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = hook,
}

return { boybomb1 = boybomb1, boybomb2 = boybomb2, boybomb3 = boybomb3 }
