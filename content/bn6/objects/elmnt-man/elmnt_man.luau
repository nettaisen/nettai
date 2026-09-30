--!strict
-- ElmntMan, the navi of the ElmntMan chips (actor object #0x10,
-- `sub_80BAA8C`), brought by their navi chip controller. He appears and
-- cycles through the four elements, a palette each, every Param1 ticks
-- until his user presses A (or 20 cycles' worth pass, then one at
-- random), and attacks with it:
--
--   Fire: a meteor on each panel with an enemy navi (objects/meteor);
--   Aqua: ice in the three panels of the column ahead (objects/elmnt-ice);
--   Elec: a lightning bolt in each row of the column three ahead
--         (objects/elmnt-bolt);
--   Wood: the field turns to grass first; then vines along his row, one
--         panel at a time, until a panel can't hold one (objects/elmnt-vine).
--
-- Then he leaves, and the controller moves on. See docs/engine/chips.md
-- §3.6.7.
--
-- The chips' script (navi chip subtype 16): `navi_chip` is his spawner,
-- `off_802CD5C[16]`.

local panels = require("../../lib/panels")
local meteor = require("../meteor/meteor")
local elmnt_ice = require("../elmnt-ice/elmnt_ice")
local elmnt_bolt = require("../elmnt-bolt/elmnt_bolt")
local elmnt_vine = require("../elmnt-vine/elmnt_vine")

export type State = {
    -- The element shown: 0 Fire, 1 Aqua, 2 Elec, 3 Wood (ExtraVars[0]).
    element: number,
    -- Fire's targets, packed panels (object +0x64).
    targets: { number },
    -- The controller waiting for him (its pointer sits in his collision
    -- data slot).
    controller: Object?,
}

local elmnt_man = {
    state = { element = "u8", targets = "u8[18]", controller = "object" },
}

local SPRITE = "08-10"

-- His actor record (the parts it adds: his overlay, `sub_8011004`).
local ACTOR_TYPE: ActorType = "player"
local AI_INDEX = 0x10

-- The elements' palettes, in the order he cycles through them
-- (`dword_80BAC4C`).
local PALETTES = { [0] = 2, 4, 8, 6 }
local FIRE, AQUA, ELEC, WOOD = 0, 1, 2, 3

-- Panel flags he needs to stay (as one word).
local CAN_STAND = 0x10010

-- The panels Fire aims at, by side (`off_80BAE58`: an enemy navi's body).
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }

-- `sub_80BAE16`: ElmntMan for `user`, reporting to `controller`.
function elmnt_man.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = battle.spawn_kind("elmnt-man", Vec3.zero, spec.params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
    o.element = spec.element
    o.related1 = user
    o.alliance, o.flip = user.alliance, user.flip
    o.damage = spec.damage
    o.stamina = bit32.rshift(spec.damage, 16)
    local s = o.state :: State
    s.controller = controller
    return o
end

-- `sub_80BAAB0`: on his panel, on the ground, with his parts.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:add_navi_parts(ACTOR_TYPE, AI_INDEX, 1)
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
        battle.play_sound(0x94)
        me.visible = true
        me.timer = 2
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    local stands = bit32.band(field.flags(me.panel_x, me.panel_y), CAN_STAND) == CAN_STAND
    me:set_action(if stands then 4 else 0x18)
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

-- `sub_80BABAC`: the next element every Param1 ticks until the user
-- presses A while dimmed; after 20 cycles' worth, one at random.
local function cycle(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 4
        s.element = FIRE
        show_element(me)
        battle.play_sound(0x134)
        me.timer = me:param(1)
        me.timer2 = me:param(1) * 20
        return
    end
    local user = me.related1 :: Object
    if user:dimmed_pressed("a") then
        battle.play_sound(0x182)
        me:set_action(0xC)
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        s.element = bit32.band(s.element + 1, 3)
        show_element(me)
        battle.play_sound(0x134)
        me.timer = me:param(1)
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
                battle.effect(Vec3.new(cx, cy, 8), 2, 0, 5)
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
            battle.play_sound(0x11B)
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
-- apart (Param2 targets, Param3 dropped).
local function fire(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 4
        local found = panels.except(me, ENEMY_NAVI[me.alliance], 0)
        if #found == 0 then
            me:set_action(0x14)
            return
        end
        for i, p in found do
            s.targets[i] = p
        end
        me:set_param(2, #found)
        me:set_param(3, 0)
        me.timer = 0
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    -- sub_80BAE60
    local n = me:param(3)
    local x, y = panels.unpack(s.targets[n + 1])
    meteor.spawn(me, x, y, 1, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)), true)
    n += 1
    me:set_param(3, n)
    if n >= me:param(2) then
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
            -- r3 (1 << 16) becomes the ice's Z; r4 its params.
            elmnt_ice.spawn(me, x, y, 2, { 0, 0x1E, 1 }, damage, 0x1_0000)
            if field.solid(x, y) then
                field.set_type(x, y, "ice")
            end
        end
        battle.play_sound(0x99)
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
    if me.phase_init == 0 then
        me.phase_init = 4
        me:set_param(3, 0)
        me.timer = 0
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    local n = me:param(3)
    local x, y = me.panel_x + me.facing * (n + 1), me.panel_y
    local vine: Object? = nil
    if field.solid(x, y) then
        -- r4 (n + 1) becomes its params; r3, left by the solidity check
        -- (`_object_getPanelDataOffset`), its Z: the row minus 1.
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        vine = elmnt_vine.spawn(me, x, y, 4, { n + 1 }, damage, y - 1)
    end
    if not vine then
        me:set_action(0x14)
        return
    end
    n += 1
    me:set_param(3, n)
    if n >= 5 then
        me:set_action(0x14)
        return
    end
    me.timer = 0xF
end

-- `sub_80BAD76`: Elec: a bolt in each row of the column three ahead
-- (`sub_80BAEC0`), 16 ticks apart.
local function elec(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me:set_param(3, 0)
        me.timer = 0
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    local n = me:param(3)
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    elmnt_bolt.spawn(me, me.panel_x + me.facing * 3, n + 1, 3, { n }, damage)
    n += 1
    me:set_param(3, n)
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
    me:remove_navi_parts(ACTOR_TYPE, AI_INDEX)
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:set_lifecycle("destroy")
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

-- `sub_80BAA8C`.
function elmnt_man.update(me: Object)
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
end

return elmnt_man
