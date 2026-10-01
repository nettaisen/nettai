--!strict
-- ElmntMan, the navi of the ElmntMan chips (the original's actor object
-- #0x10, `sub_80BAA8C`), brought by their navi chip controller. He appears
-- and cycles through the four elements, a palette each, every few ticks
-- (the chip's) until his user presses A (or 20 cycles' worth pass, then
-- one at random), and attacks with it:
--
--   Fire: a meteor on each panel with an enemy navi (./meteor);
--   Aqua: ice in the three panels of the column ahead (./ice);
--   Elec: a lightning bolt in each row of the column three ahead (./bolt);
--   Wood: the field turns to grass first; then vines along his row, one
--         panel at a time, until a panel can't hold one (./vine).
--
-- Then he leaves, and the controller moves on. See docs/engine/chips.md
-- §3.6.7.

local panels = require("../../lib/panels")
local bolt = require("./bolt")
local effects = require("../../lib/effects")
local ice = require("./ice")
local meteor = require("./meteor")
local vine = require("./vine")
local navi_chips = require("../../lib/navi-chips/navi")

-- What a chip summons him with: the ticks each element shows (the chips'
-- first parameter).
export type Summon = { cycle_ticks: number }

export type State = {
    -- The ticks each element shows (Param1).
    cycle_ticks: number,
    -- Fire's targets (Param2), and how many meteors, bolts or vines are
    -- out (Param3).
    count: number,
    sent: number,
    -- The element shown: 0 Fire, 1 Aqua, 2 Elec, 3 Wood (ExtraVars[0]).
    element: number,
    -- Fire's targets, packed panels (object +0x64).
    targets: { number },
    -- The controller waiting for him (its pointer sits in his collision
    -- data slot).
    controller: Object?,
}

local elmntman = {}

local SPRITE = asset.sprite("elmnt-man")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_ELMNT_MAN = asset.sound("elmnt-man")
local SOUND_ELMNT_MAN_2 = asset.sound("elmnt-man-2")
local SOUND_ELMNT_MAN_3 = asset.sound("elmnt-man-3")
local SOUND_ELMNT_MAN_4 = asset.sound("elmnt-man-4")

-- What he is taken for: his actor record, and what he wears (his overlay, `sub_8011004`).
local IDENTITY = define.identity {
    id = "elementman",
    class = "navi",
    version = 0,
    ai_index = 0x10,
    ice = "large",
    parts = {
        body = {
            sprite = asset.sprite("elmnt-man"),
            anims = 32,
            behind = { { 0x00, 0x0B }, { 0x1F, 0x1F } },
            own_palette = true,
            anim_offset = 0x09,
        },
    },
}

-- The elements' palettes, in the order he cycles through them
-- (`dword_80BAC4C`).
local PALETTES = { [0] = 2, 4, 8, 6 }
local FIRE, AQUA, ELEC, WOOD = 0, 1, 2, 3

-- How long Aqua's ice stands.
local ICE_TICKS = 0x1E

-- The panels Fire aims at, by side (`off_80BAE58`: an enemy navi's body).
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }

-- `sub_80BAAB0`: on his panel, on the ground, with his parts.
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "ground", true)
    me:add_parts(IDENTITY, 1)
    me:set_lifecycle("update")
end

-- Enter the phase with a timer, or count it down; true once it is 0 or
-- below.
local function countdown(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = ticks
        return false
    end
    local left = me.timer - 1
    me.timer = left
    return left <= 0
end

local function show_element(me: Object)
    me.sprite.palette = PALETTES[(me.state :: State).element]
end

-- `sub_80BAB30`: he appears; 2 ticks; on to standing, or straight to
-- leaving if his panel can't hold him.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
        me.timer = 2
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me:set_action(if navi_chips.has_footing(me) then 4 else 0x18)
end

-- `sub_80BAB84`: 10 ticks standing.
local function stand(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
    end
    if countdown(me, 0xA) then
        me:set_action(8)
    end
end

-- `sub_80BABAC`: the next element every few ticks until the user
-- presses A while dimmed; after 20 cycles' worth, one at random.
local function cycle(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 4
        s.element = FIRE
        show_element(me)
        battle.play_sound(SOUND_ELMNT_MAN)
        me.timer = s.cycle_ticks
        me.timer2 = s.cycle_ticks * 20
        return
    end
    local user = me.related1 :: Object
    if user:dimmed_pressed("a") then
        battle.play_sound(SOUND_ELMNT_MAN_2)
        me:set_action(0xC)
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        s.element = bit32.band(s.element + 1, 3)
        show_element(me)
        battle.play_sound(SOUND_ELMNT_MAN)
        me.timer = s.cycle_ticks
    end
    left = me.timer2 - 1
    me.timer2 = left
    if left > 0 then
        return
    end
    s.element = bit32.band(battle.rng_positive(), 3)
    show_element(me)
    me:set_action(0xC)
end

-- `sub_80BAF06`: Wood's grass: every solid panel turns to grass, with a
-- sparkle.
local function grass(me: Object)
    for x = 1, 6 do
        for y = 1, 3 do
            if field.solid(x, y) then
                field.set_type(x, y, "grass")
                local cx, cy = field.center(x, y)
                battle.effect(Vec3.new(cx, cy, 8), effects.ripple, 0, 5)
            end
        end
    end
end

-- `sub_80BAC50`: 35 ticks of winding up (Wood turns the field to grass
-- first), raising his arms at 16 left; then the element's attack.
local function wind_up(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = 5
        me.timer = 0x23
        if s.element == WOOD then
            grass(me)
            battle.play_sound(SOUND_ELMNT_MAN_3)
        end
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0x10 then
        me.anim = 7
        return
    end
    if left > 0 then
        return
    end
    me:set_action(0x10)
    me.phase = s.element * 4
end

-- `sub_80BACBC`: Fire: a meteor on each target (`sub_80BAE3C`), 12 ticks
-- apart.
local function fire(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 4
        local found = panels.filtered_except(me, ENEMY_NAVI[me.alliance], 0)
        if #found == 0 then
            me:set_action(0x14)
            return
        end
        for i, p in found do
            s.targets[i] = p
        end
        s.count = #found
        s.sent = 0
        me.timer = 0
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    -- sub_80BAE60
    local n = s.sent
    local p = s.targets[n + 1]
    local x, y = panels.x(p), panels.y(p)
    meteor.spawn(me, x, y, 1, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)), true)
    n += 1
    s.sent = n
    if n >= s.count then
        me:set_action(0x14)
        return
    end
    me.timer = 0xC
end

-- `sub_80BAD06`: Aqua: ice in the column ahead (`sub_80BAE7A`), each
-- solid panel of it turned to ice; 30 ticks.
local function aqua(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        for dy = -1, 1 do
            local x, y = me.panel_x + me.facing, me.panel_y + dy
            -- r3 (1 << 16) becomes the ice's Z.
            ice.spawn(me, x, y, 2, ICE_TICKS, true, damage, 0x1_0000)
            if field.solid(x, y) then
                field.set_type(x, y, "ice")
            end
        end
        battle.play_sound(SOUND_ELMNT_MAN_4)
        me.timer = 0x1E
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(0x14)
    end
end

-- `sub_80BAD34`: Wood: a vine a panel further along his row every 15
-- ticks (`sub_80BAEDC`), five at most, until a panel isn't solid (or no
-- vine could spawn).
local function wood(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 4
        s.sent = 0
        me.timer = 0
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    local n = s.sent
    local x, y = me.panel_x + me.facing * (n + 1), me.panel_y
    local grown: Object? = nil
    if field.solid(x, y) then
        -- r3, left by the solidity check (`_object_getPanelDataOffset`),
        -- becomes its Z: the row minus 1.
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        grown = vine.spawn(me, x, y, 4, damage, y - 1)
    end
    if not grown then
        me:set_action(0x14)
        return
    end
    n += 1
    s.sent = n
    if n >= 5 then
        me:set_action(0x14)
        return
    end
    me.timer = 0xF
end

-- `sub_80BAD76`: Elec: a bolt in each row of the column three ahead
-- (`sub_80BAEC0`), 16 ticks apart.
local function elec(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 4
        s.sent = 0
        me.timer = 0
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    local n = s.sent
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    bolt.spawn(me, me.panel_x + me.facing * 3, n + 1, 3, damage)
    n += 1
    s.sent = n
    if n >= 3 then
        me:set_action(0x14)
        return
    end
    me.timer = 0x10
end

local ATTACKS = { [0] = fire, [4] = aqua, [8] = elec, [0xC] = wood }

-- `sub_80BAC98`: the element's attack, by phase.
local function attack(me: Object)
    local f = ATTACKS[me.phase]
    if not f then
        error(string.format("ElmntMan's attack %#x reads past its table (off_80BACAC)", me.phase))
    end
    f(me)
end

-- `sub_80BADB4`: 20 ticks, back to standing at 5 left.
local function wind_down(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = 0x14
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 5 then
        me.anim = 8
        return
    end
    if left <= 0 then
        me:set_action(0x18)
    end
end

-- `sub_80BADE4`: 2 ticks, then he goes (his parts with him) and the
-- controller moves on.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    if not countdown(me, 2) then
        return
    end
    me:remove_parts(IDENTITY)
    navi_chips.leave(me, (me.state :: State).controller)
end

local ACTIONS = {
    [0] = appear,
    [4] = stand,
    [8] = cycle,
    [0xC] = wind_up,
    [0x10] = attack,
    [0x14] = wind_down,
    [0x18] = leave,
}

elmntman.kind = define.kind {
    id = "elmntman/navi",
    pool = "actor",
    state = {
        element = "u8",
        targets = "u8[18]",
        controller = "object",
        cycle_ticks = "u8",
        count = "u8",
        sent = "u8",
    },
    -- `sub_80BAA8C`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("ElmntMan action %#x reads past his table (off_80BAB14)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80BAE16`: the ElmntMan chips' navi hook (`off_802CD5C[16]`):
-- ElmntMan for `user`, reporting to `controller`, each element showing
-- `cycle_ticks`. (His position is the spawner's registers, all of which
-- his init overwrites.)
function elmntman.summon(args: Summon): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(elmntman.kind, user, spec)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.cycle_ticks = args.cycle_ticks
        return o
    end
end

return elmntman
