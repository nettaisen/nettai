--!strict
-- The patch cards' ZapRng (`megaman/zap-ring`, B↓ZapRng, Bunny's card): weapon routine 0x42 (0x0800FB76) and action
-- 0x32 (0x080EFFC8, EXE5's own): the navi holds out its ring (the attachment table's row 0x38, 0c-5a, at its attach
-- point 0x1C) and, five ticks after the first, sends a ring (attack object #0xC4, 0x080DF38C: EXE6's code,
-- `sub_80C51CC` and `sub_80C524C`, which no EXE6 content spawns, EXE5's look) along its row from the panel ahead, 26
-- pixels up: Elec, 30 damage, hit parameter 10, paralyzing. 11 ticks in, 11 more, and it is back to idle.
--
-- The ring: by its first parameter (the attack's parameters' first byte: 0), a row of 0x080DF380 (its speed in
-- pixels a tick, the status its hits give, its sound: 10, paralysis for 90 ticks, 0xB3); an attack hitting navis,
-- hit modifier 0, its element's spark. It flies ahead until it hits something, leaves the field or the battle ends.

local attachment = require("../../../objects/attachment")
local collision = require("../../../rules/collision")
local sparks = require("../../../lib/sparks")
local statuses = require("../../../rules/status")

-- ---- The ring -------------------------------------------------------------------------------------------------------

local SPRITE = asset.sprite("zap-ring")
-- Its row (0x080DF380's row 0, the ZapRng attack's first parameter): its speed (pixels a tick), the status its hits
-- give, its sound.
local SPEED, STATUS, SOUND = 10, statuses.paralyze_90, asset.sound("aqua-needle-2")

-- The end: no region, hidden, destroyed next update.
local function finish(me: Object)
    me.collision:set_region(nil)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- 0x080DF42C (EXE6's `sub_80C524C`): its hits; on a hit or the battle's end it goes; else it flies on, leaving the
-- field it goes.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 or battle.over() then
        return finish(me)
    end
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if not field.valid(me.panel_x, me.panel_y) then
        return finish(me)
    end
    c:present()
    me:update_sprite()
end

-- 0x080DF3AC (EXE6's `sub_80C51CC`, EXE5's sheet): shown on its panel (its height the spawner's), flying ahead at its
-- row's speed; an attack hitting navis with its row's status and its element's spark; its row's sound.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:step()
    sprite.shadow = "with_sprite"
    me:set_coordinates_from_panel()
    me.vel = Vec3.new(int.shl(me.facing * SPEED, 16), me.vel.y, me.vel.z)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, 0)
    me.collision.status_base = STATUS
    me.collision:set_hit_effect(sparks.of_element(me.element))
    me.collision:present()
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
end

local RING = new.kind {
    id = "megaman/zap-ring/ring",
    pool = "attack",
    -- 0x080DF38C.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
    end,
}

-- 0x080DF47A: a ring from (x, y) with the element, height and damage word, of `owner`'s side and facing. (Its
-- position is the spawner's registers until its init places it: its row, its element and its height.)
local function send(owner: Object, x: number, y: number, element: number, z: number, damage: number): Object?
    local o = battle.spawn(RING, Vec3.new(y, element, z))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o:set_damage_word(damage)
    o.alliance, o.flip = owner.alliance, owner.flip
    o.related1 = owner
    return o
end

-- ---- The action -----------------------------------------------------------------------------------------------------

local ARM = attachment.look { sprite = asset.sprite("zap-ring-arm"), palette = 0, attach_point = 0x1C }
local ANIM = 10
local HOLD_TICKS, SEND_AT, RECOVER_TICKS = 10, 5, 10
local RING_Z = 0x1A_0000

export type State = {
    -- AIAttackVars+0x10.
    timer: number,
}

-- 0x080EFFE4: the ring held out; ten ticks after, the ring at 5; then on.
local function hold(me: Object, s: State)
    if me.step_init == 0 then
        me:set_status("using_action", true)
        me:set_animation(ANIM)
        me:open_counter_window()
        attachment.attach(me, ARM, "overlay")
        s.timer = HOLD_TICKS
        me.step_init = 4
        return
    end
    if s.timer == SEND_AT then
        -- (The damage word takes the Atk+ bonus, AIAttackVars+6; the ring's row is the parameters' first byte: 0.)
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        send(me, me.panel_x + me.facing, me.panel_y, me.attack_element, RING_Z, damage)
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t < 0 then
        me.step, me.step_init = 4, 0
    end
end

-- 0x080F0046: 11 ticks, then the action's flag off and back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t < 0 then
        me:set_status("using_action", false)
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end
end

local ACTION = new.action {
    id = "megaman/zap-ring/action",
    state = { timer = "u16" },
    -- 0x080EFFC8.
    update = function(me: Object, s: State)
        if me.step == 0 then
            hold(me, s)
        elseif me.step == 4 then
            recover(me, s)
        else
            error(string.format("ZapRng's phase %#x reads past 0x080EFFDC", me.step))
        end
    end,
}

-- 0x0800FB76: 30 damage, hit parameter 10, Elec; no charge, bonus or lockout; parameters 0.
return new.weapon {
    id = "megaman/zap-ring",
    -- The charge table's row 0x42, by Charge 0 to 5: past 4 it reads on into row 0x43's first entry.
    charge_ticks = { 90, 85, 80, 75, 70, 60 },
    setup = function(navi: Object): Action
        navi.attack_damage = 30
        navi.hit_param = 10
        navi.attack_element = 3
        navi.charged = 0
        navi.extra = 0
        navi.attack_lockout = 0
        return ACTION
    end,
}
