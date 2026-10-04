--!strict
-- BN5's CsmoPris (the navi chips' action with the summon table's entry 22, 0x080BFA4A): BN5's own CosmoMan (actor
-- object #0x26, 0x080BF814), brought by the navi chip controller. He appears (and goes at once off a solid panel's
-- want), poses, rises over the middle row, and sends nine comets (attack object #0x74, 0x080D482C: its variant 4)
-- from 20 pixels ahead of him along the rows in turn, 20 ticks apart; 60 ticks later he comes back down and goes,
-- and the controller moves on. A comet flies ahead 4 pixels a tick, 16 pixels up; at each panel's center, with no
-- enemy navi straight ahead but one a row up or down, it turns toward it; it hits navis (hit modifier 3, the heavy
-- spark) and goes off the field.
--
-- (The comet's variants 0 to 3 are CosmoMan the virus's: they wait out the dimming and fly straight. In the operation
-- battle (the operation battle), never in a netbattle, a comet's speed is its own row's, 0x080D4948. Neither is met in a netbattle; both
-- are ported as the code has them.)

local collision = require("../../rules/collision")
local navi_chips = require("@exelib/navi-chips/navi")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local operation_battle = require("../../lib/operation_battle")

local csmopris = {}

local SPRITE = asset.sprite("cosmoman")

-- ---- The comet ------------------------------------------------------------------------------------------------------

-- Its speed by its variant (Param1) and row (Param2): 0x080D491C's rows of five, or 0x080D4948 in the operation battle
-- mode; its turn's speed by its row (0x080D4B54); its HP by its variant (0x080D4820).
local SPEEDS: { [number]: { [number]: number } } = {
    [0] = { [0] = 0x4_1999, 0x4_6666, 0x4_B333, 0x5_0000, 0x4_0000 },
    [1] = { [0] = 0x3_6666, 0x3_B333, 0x4_0000, 0x4_4CCC, 0x4_0000 },
}
local SPECIAL_SPEEDS: { [number]: number } = { [0] = 0x3_1999, 0x2_6666 }
local TURN_SPEEDS: { [number]: number } = { [0] = 0x2_6666, 0x2_6666 }
local HP = 10
local HIT_MOD = 3
-- The enemy navis a comet turns toward (0x080D4B48).
local TARGETS = { [0] = 0x0400_0000, [1] = 0x0800_0000 }
local SOUNDS = { [0] = asset.sound("sound-138"), [1] = asset.sound("sound-137") }

export type CometState = {
    -- Its variant (Param1: 0 to 3 the virus's, 4 the navi chip's) and row (Param2).
    variant: number,
    row: number,
}

-- Action 0 (0x080D49C0): its sound; 5 ticks; then its flying animation and its hit, and on.
local function start(me: Object, s: CometState)
    if me.phase == 0 then
        me.phase = 4
        battle.play_sound(SOUNDS[if s.row ~= 0 then 1 else 0])
        me.timer = 5
        return
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t >= 0 then
        return
    end
    me.anim = if s.row ~= 0 then 18 else 17
    me.collision:set_region(regions.single)
    me:set_action(if s.variant < 4 then 4 else 8)
end

-- Off the field: its hit off, gone.
local function off_field(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        me.collision:set_region(nil)
        -- (The state byte alone, as every destroy of the comet's: 0x080D4A5C, 0x080D4AB8.)
        me.lifecycle = "destroy"
    end
end

-- On a new panel, its hit there.
local function moved(me: Object, x: number)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if me.panel_x ~= x then
        me.collision:set_region(regions.single)
    end
end

-- Action 4 (0x080D4A16, the virus's): straight ahead. (In the operation battle it also marks the panel ahead,
-- 0x080E9FA4's effect object #0x83, not ported.)
local function straight(me: Object)
    local x = me.panel_x
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    moved(me, x)
    off_field(me)
end

-- 0x080D4AF6: at a panel's center, with no enemy navi straight ahead, toward one a row up, or (rather) down.
local function turn(me: Object, s: CometState)
    local mask = TARGETS[me.alliance]
    local speed = TURN_SPEEDS[s.row] or error(string.format("the comet's row %d reads past 0x080D4B54", s.row))
    local x = me.panel_x + me.facing
    if bit32.btest(field.flags(x, me.panel_y), mask) then
        return
    end
    if bit32.btest(field.flags(x, me.panel_y - 1), mask) then
        me.vel = Vec3.new(me.vel.x, -speed, me.vel.z)
    end
    if bit32.btest(field.flags(x, me.panel_y + 1), mask) then
        me.vel = Vec3.new(me.vel.x, speed, me.vel.z)
    end
end

-- Action 8 (0x080D4A60, the navi chip's): ahead (and up or down once turned); crossing a panel's center before it has
-- turned, it may turn.
local function homing(me: Object, s: CometState)
    local center = field.center(me.panel_x, me.panel_y)
    local x = me.panel_x
    local p, v = me.pos, me.vel
    local old_x = p.x
    me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), p.z)
    if v.y == 0 and center ~= old_x and navi_chips.between(me.pos.x, old_x, center) then
        turn(me, s)
    end
    moved(me, x)
    off_field(me)
end

-- 0x080D4950: the hits against it resolve and spark; at the battle's end its hit off and gone; a body's hit takes its
-- hit off; the virus's wait out the dimming (and so does any once a body has hit it).
local function comet_tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c:set_region(nil)
        -- (0x080D49A6: the state byte alone.)
        me.lifecycle = "destroy"
        return
    end
    local s = me.state :: CometState
    local wait = s.variant < 4
    if bit32.btest(c.hit_flags, 0x0F80_0000) then
        c:set_region(nil)
        wait = true
    end
    if not (wait and battle.dimmed()) then
        local a = me.action
        if a == 0 then
            start(me, s)
        elseif a == 4 then
            straight(me)
        elseif a == 8 then
            homing(me, s)
        else
            error(string.format("CsmoPris's comet: action %#x reads past 0x080D49B0", a))
        end
    end
    c:present()
end

-- 0x080D4860: its panel from where it is; shown, 08-10's animation 16 with a ground shadow; its speed; its collision
-- (row 0x13, hitting what row 0x14 reaches: hit modifier 3, the heavy spark; no hit yet); guarding; its HP.
local function comet_init(me: Object)
    me:set_panel_from_coordinates()
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 16, 16
    sprite:set_animation(16)
    local s = me.state :: CometState
    local speed
    if operation_battle() then
        speed = SPECIAL_SPEEDS[s.row]
    else
        local row = SPEEDS[s.row]
        speed = row and row[s.variant]
    end
    if not speed then
        error(string.format("the comet's variant %d, row %d read past its speed table", s.variant, s.row))
    end
    me.vel = Vec3.new(int.mul32(me.facing, speed), 0, me.vel.z)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.own_body, collision.hit_by_other_side, HIT_MOD)
    me.collision:set_hit_effect(sparks.heavy)
    me.collision:set_region(nil)
    me.collision:present()
    me:set_status("guard", true)
    me.hp, me.max_hp = HP, HP
    me:set_lifecycle("update")
end

csmopris.comet = define.kind {
    id = "csmopris/comet",
    pool = "attack",
    state = { variant = "u8", row = "u8" },
    -- 0x080D482C: by its lifecycle; its sprite steps (while dimmed too for the navi chip's).
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            comet_init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            comet_tick(me)
        else
            me:destroy()
            return
        end
        if (me.state :: CometState).variant == 4 then
            me:update_sprite_while_dimmed()
        else
            me:update_sprite()
        end
    end,
}

-- 0x080D4AC4: a comet at `pos` (16 pixels up) of `variant` and `row`, from `owner`, with the element and damage word,
-- of its side and facing, running while dimmed.
local function spawn_comet(owner: Object, x: number, y: number, element: number, variant: number, row: number, damage: number): Object?
    local o = battle.spawn(csmopris.comet, Vec3.new(x, y, 0x10_0000))
    if not o then
        return nil
    end
    o.element = element
    o.related1 = owner
    o:set_damage_word(damage)
    o.alliance, o.flip = owner.alliance, owner.flip
    o.run_while_dimmed = true
    local s = o.state :: CometState
    s.variant, s.row = variant, row
    return o
end

-- ---- CosmoMan -------------------------------------------------------------------------------------------------------

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x60).
    controller: Object?,
    -- The comets sent (Param2).
    sent: number,
}

local SOUND_APPEAR, SOUND_POSE = asset.sound("appear"), asset.sound("sound-13a")
local COMETS, EVERY, AHEAD = 9, 20, 20
local SOLID = 0x10

-- Enter the phase with a timer, or count down; true once it has run out.
local function countdown(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = ticks
        return false
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left <= 0
end

-- He is done: the controller moves on, and he goes.
local function done(me: Object)
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:set_lifecycle("destroy")
end

-- Action 0 (0x080BF8C4): he appears; 10 ticks; then he stays, or goes off a panel that isn't solid.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
        me.timer = 10
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    me:set_action(if field.check(me.panel_x, me.panel_y, SOLID, 0) then 4 else 0x14)
end

-- Action 4 (0x080BF914): his pose (animation 8); 20 ticks.
local function pose(me: Object)
    if me.phase_init == 0 then
        me.anim = 8
        battle.play_sound(SOUND_POSE)
    end
    if countdown(me, 20) then
        me:set_action(8)
    end
end

-- Action 8 (0x080BF944): he rises (animation 9) over the middle row, keeping his panel; 20 ticks.
local function rise(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = 9
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        me:reserve_panel(me.panel_x, me.panel_y)
        me.panel_y = 2
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x, int.i32(p.y + 0x20_0000), int.i32(p.z + 0x20_0000))
        me.timer = 20
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    me.timer = 20
    ;(me.state :: State).sent = 0
    me:set_action(0xC)
end

-- Action 0xC (0x080BF996): every 20 ticks a comet from 20 pixels ahead, along the rows in turn; nine.
local function comets(me: Object)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    local s = me.state :: State
    local _, y = field.center(me.panel_x, s.sent % 3 + 1)
    local x = int.i32(me.pos.x + int.shl(AHEAD * me.facing, 16))
    spawn_comet(me, x, y, me.element, 4, 1, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
    s.sent += 1
    if s.sent < COMETS then
        me.timer = EVERY
    else
        me:set_action(0x10)
    end
end

-- Action 0x10 (0x080BF9E8): 60 ticks.
local function wait(me: Object)
    if countdown(me, 60) then
        me:set_action(0x14)
    end
end

-- Action 0x14 (0x080BFA0C): back down on his panel (animation 4); 3 ticks; he goes.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
        me:unreserve_panel(me.panel_x, me.panel_y)
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
        me.anim = 4
        me.timer = 3
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        done(me)
    end
end

local ACTIONS = { [0] = appear, [4] = pose, [8] = rise, [0xC] = comets, [0x10] = wait, [0x14] = leave }

csmopris.kind = define.kind {
    id = "csmopris/navi",
    pool = "actor",
    state = { controller = "object", sent = "u8" },
    -- 0x080BF814.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080BF838: on his panel, on the ground, 08-10 with a ground shadow, palette 0 (0x0800DA8A's for
            -- level 0), facing his way.
            navi_chips.stand(me, SPRITE, "ground", true)
            me.sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me:set_lifecycle("update")
        elseif l == "update" then
            -- 0x080BF886: at the battle's end the controller moves on and he goes.
            if battle.over() then
                done(me)
            else
                local action = ACTIONS[me.action]
                if not action then
                    error(string.format("CsmoPris's CosmoMan: action %#x reads past 0x080BF8AC", me.action))
                end
                action(me)
            end
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BFA4A: CsmoPris's CosmoMan for `user`, reporting to `controller`.
function csmopris.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = navi_chips.spawn(csmopris.kind, user, spec)
    if not o then
        return nil
    end
    (o.state :: State).controller = controller
    return o
end

return csmopris
