--!strict
-- BN5's HotBody1-3 (action 0x58, 0x080F013A): BN5's own. The navi stands (animation 0) while its fire (effect object
-- #0x57, 0x080E6B88) burns: a flame (attack object #0x9D, 0x080DA390) on the navi's panel, then every
-- 30 ticks, for each panel the last flames stood on, a flame on each of the eight panels around it that is solid,
-- holds one of the other side's bodies or objects or a neutral object (0x080E6CC4), and has had none; it goes when
-- its rounds (the record's second parameter: 2) are done or a round lights nothing, and lets the navi go: 6 ticks
-- later it is back to idle.

local collision = require("../../rules/collision")

local hotbody = {}

-- ---- The flame ------------------------------------------------------------------------------------------------------

-- Its look (`sub_8011504`'s word 0x00001040: sheet 10-40, animation 0, no shadow) and collision (0x04050300: an attack
-- that hits navis, hit modifier 3; its region its panel from the start); its sound.
local SPRITE = asset.sprite("hot-body-flame")
local SOUND = asset.sound("form-change")
local HIT_MOD = 3
-- Its actions: rising (4 ticks), burning (30 ticks, animation 1), dying down (9 ticks, animation 2).
local RISE, BURN, DIE = 0, 4, 8
local RISE_TICKS, BURN_TICKS, DIE_TICKS = 4, 30, 9

-- 0x080DA410.
local function rise(me: Object)
    if me.phase_init == 0 then
        me.timer = RISE_TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t <= 0 then
        me:set_action(BURN)
    end
end

-- 0x080DA432.
local function burn(me: Object)
    if me.phase_init == 0 then
        me.anim = 1
        me.timer = 0
        me.phase_init = 4
    end
    local t = me.timer + 1
    me.timer = t
    if t >= BURN_TICKS then
        me:set_action(DIE)
    end
end

-- 0x080DA45A: its hit off, gone.
local function die(me: Object)
    if me.phase_init == 0 then
        me.anim = 2
        me.timer = DIE_TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t <= 0 then
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end
end

-- 0x080DA3DE: a hit takes its region off; its action.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local a = me.action
    if a == RISE then
        rise(me)
    elseif a == BURN then
        burn(me)
    elseif a == DIE then
        die(me)
    else
        error(string.format("HotBody's flame: action %#x reads past 0x080DA404", a))
    end
    c:present()
end

-- 0x080DA3B4: on its panel's center, on the ground; its look and collision; the sound; the first update at once.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    battle.play_sound(SOUND)
    me.lifecycle = "update"
    tick(me)
end

hotbody.flame = define.kind {
    id = "hotbody/flame",
    pool = "attack",
    -- 0x080DA390: by its lifecycle, then a sprite step.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- ---- The fire -------------------------------------------------------------------------------------------------------

-- What a flame spreads onto (0x080E6CC4): the other side's bodies and objects, and neutral objects; and solid.
local SPREADS_ONTO = { [0] = 0x0580_0000, [1] = 0x0A80_0000 }
local SOLID = 0x10
-- The eight panels around one (0x080E6CCC), clockwise from the one above.
local AROUND = { { 0, -1 }, { 1, -1 }, { 1, 0 }, { 1, 1 }, { 0, 1 }, { -1, 1 }, { -1, 0 }, { -1, -1 } }
local EVERY = 30
-- The list of the last round's flames holds 8 (+0x78); more would run into the object's next fields.
local MAX_LIT = 8

export type FireState = {
    -- The panels lit, a bit a panel by row (+0x60: a nibble a panel, rows 0 to 4).
    lit: { number },
    -- The last round's flames (+0x78) and how many (+0x74).
    last: { number },
    count: number,
    -- The navi it lets go (its +0x40: the address of the navi's related1).
    navi: Object?,
    -- Its rounds (its second parameter).
    rounds: number,
}

local function is_lit(s: FireState, x: number, y: number): boolean
    return bit32.btest(s.lit[y + 1], bit32.lshift(1, x))
end

-- 0x080E6D08: a panel lit, listed, and a flame on it with the fire's element, height, parameters and damage word.
local function light(me: Object, x: number, y: number)
    local s = me.state :: FireState
    s.lit[y + 1] = bit32.bor(s.lit[y + 1], bit32.lshift(1, x))
    if s.count >= MAX_LIT then
        error("HotBody's fire: a ninth flame in a round runs its list (+0x78) into the object's next fields")
    end
    s.count += 1
    s.last[s.count] = bit32.bor(x, bit32.lshift(y, 4))
    -- 0x080DA480: its position is the spawner's registers until its init places it.
    local o = battle.spawn(hotbody.flame, Vec3.new(y, me.element, me.pos.z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = me.element
        o:set_damage_word(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
        o.alliance, o.flip = me.alliance, me.flip
        o.related1 = me
    end
end

-- 0x080E6C5C: from (x, y), the panels around it the fire spreads onto.
local function spread_from(me: Object, x: number, y: number)
    local s = me.state :: FireState
    local onto = SPREADS_ONTO[me.alliance]
    for _, d in AROUND do
        local px, py = x + d[1], y + d[2]
        local f = field.flags(px, py)
        if f ~= 0 and bit32.btest(f, onto) and bit32.btest(f, SOLID) and not is_lit(s, px, py) then
            light(me, px, py)
        end
    end
end

-- 0x080E6C2A: a round: from each of the last round's flames.
local function spread(me: Object)
    local s = me.state :: FireState
    local n = s.count
    local from: { number } = {}
    for i = 1, n do
        from[i] = s.last[i]
    end
    s.count = 0
    for i = 1, n do
        local p = from[i]
        spread_from(me, bit32.band(p, 7), bit32.rshift(p, 4))
    end
end

-- Done: the navi let go, gone.
local function finish(me: Object)
    local s = me.state :: FireState
    local navi = s.navi
    if navi then
        navi.related1 = nil
    end
    me:set_lifecycle("destroy")
end

-- 0x080E6BCE: its panel from its position; every 30 ticks a round (the first at once); done after its rounds or a
-- round that lit nothing. (It means to follow the navi, copying the navi's position, but stores it at address 0x34,
-- not its own +0x34: its position stays the spawner's registers, and its panel the one they make.)
local function update(me: Object)
    me:set_panel_from_coordinates()
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t > 0 then
        return
    end
    me.timer = EVERY
    local left = me.timer2 - 1
    me.timer2 = int.u16(left)
    if left < 0 then
        finish(me)
        return
    end
    spread(me)
    if (me.state :: FireState).count == 0 then
        finish(me)
    end
end

-- 0x080E6BA8: nothing lit; its rounds (its second parameter); its own panel lit.
local function init_fire(me: Object)
    local s = me.state :: FireState
    for i = 1, 5 do
        s.lit[i] = 0
    end
    for i = 1, MAX_LIT do
        s.last[i] = 0
    end
    s.count = 0
    me.timer2 = s.rounds
    light(me, me.panel_x, me.panel_y)
    me:set_lifecycle("update")
    update(me)
end

hotbody.fire = define.kind {
    id = "hotbody/fire",
    pool = "effect",
    state = { lit = "u32[5]", last = "u8[8]", count = "u32", navi = "object", rounds = "u8" },
    -- 0x080E6B88.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init_fire(me)
        elseif l == "update" then
            update(me)
        else
            me:free()
        end
    end,
}

-- ---- The action -----------------------------------------------------------------------------------------------------

export type State = {
    -- Ticks left in the recovery (AIAttackVars+0x10).
    timer: number,
}

local STAND, RECOVER = 0x0, 0x4
local RECOVER_TICKS = 5

-- HotBody's action: the fire burns `rounds` rounds (the record's second parameter).
function hotbody.action(id: string, rounds: number): Action
    -- 0x080F0154: the fire out (0x080E6C0C: on the navi's panel, the chip's element, damage word plus the Atk+
    -- bonus; the navi's related1 it); the navi stands; until the fire lets it go.
    local function stand(me: Object, s: State)
        if me.step_init == 0 then
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            -- (Its position is the spawner's registers: the navi's row, the element and the bonus.)
            local o = battle.spawn(hotbody.fire, Vec3.new(me.panel_y, me.attack_element, me.extra))
            if o then
                o.panel_x, o.panel_y = me.panel_x, me.panel_y
                o.element = me.attack_element
                o:set_damage_word(damage)
                o.alliance, o.flip = me.alliance, me.flip
                o.related1 = me
                local fs = o.state :: FireState
                fs.navi, fs.rounds = me, rounds
                me.related1 = o
            end
            me:set_animation(0)
            me:open_counter_window()
            me:set_status("using_action", true)
            s.timer = 0
            me.step_init = 4
        end
        if me.related1 == nil then
            me.step, me.step_init = RECOVER, 0
        end
    end

    -- 0x080F019C: 6 ticks, then back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me.related1 = nil
            me.overlay = nil
            me:exit_attack()
        end
    end

    -- 0x080F013A.
    local function update_action(me: Object, s: State)
        local step = me.step
        if step == STAND then
            stand(me, s)
        elseif step == RECOVER then
            recover(me, s)
        elseif step == 0x8 then
            me:exit_attack()
        else
            error(string.format("HotBody phase %#x reads past 0x080F014C", step))
        end
    end

    return define.action { id = id, state = { timer = "u16" }, args = { rounds = rounds }, update = update_action }
end

return hotbody
