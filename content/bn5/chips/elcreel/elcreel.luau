--!strict
-- BN5's ElcReel1-3 (action 0x23, 0x080ECE4C) and their reel (attack object #0xA4, 0x080DABBC): BN5's own. The navi
-- raises its reel launcher and, on the action's second tick, the reel leaves from the panel ahead, 26 pixels up,
-- rolling 8 pixels a tick along the row and hitting navis (hit modifier 1). When it hits something it stops, hidden,
-- and 5 ticks later hooks every navi beside it, above, below, in front or behind (by the field, not by its facing):
-- a hit on each with a spark of its own, and a hook over each; then it shocks its panel once every 5 ticks, as many
-- times as it hooked navis, and is gone. The three differ in their records alone.

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local elcreel = {}

-- The reel's look: sheet 10-3e, animation 2, no shadow (0x080DABE6, `sub_8011504`'s word 0x0002103E).
local SPRITE = asset.sprite("elec-reel")
local ANIM = 2
-- Its collision (0x0A050100): thrown, hitting navis, hit modifier 1.
local HIT_MOD = 1
-- Rolling 8 pixels a tick; gone past 150 pixels either way.
local SPEED, EDGE = 0x8_0000, 150
-- The hooks (0x080DAD8C: dx, dy, the effect), and what they catch (0x080DAD84, by side: the other side's navis and
-- the field's obstacles). Each hook's hit: the panel alone, the elec spark (row 3), thrown, hitting navis, hit
-- modifier 1 (0x080DAD98's 0x0A050301).
type Hook = { dx: number, dy: number, effect: Effect }
local VERTICAL = define.effect { sprite = SPRITE, anim = 1, palette = 0 }
local HORIZONTAL = define.effect { sprite = SPRITE, anim = 2, palette = 0 }
local HOOKS: { Hook } = {
    { dx = 0, dy = -1, effect = VERTICAL },
    { dx = 1, dy = 0, effect = HORIZONTAL },
    { dx = 0, dy = 1, effect = VERTICAL },
    { dx = -1, dy = 0, effect = HORIZONTAL },
}
local CATCHES: { [number]: number } = { [0] = 0x0580_0000, [1] = 0x0A80_0000 }
local HOOK_LIFETIME = 20
-- Waiting before the hooks, and between the shocks.
local WAIT = 5

-- Its actions.
local ROLL, CAUGHT, SHOCK = 0, 4, 8

-- Done: off its panels, and freed at the next update.
local function finish(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080DAC38: rolling; a hit stops it, hidden (its collision off the field).
local function roll(me: Object)
    if me.collision.hit_flags ~= 0 then
        me.collision:set_region(nil)
        me.visible = false
        me.action, me.phase, me.phase_init = CAUGHT, 0, 0
        return
    end
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if int.u32(int.asr(me.pos.x, 16) + EDGE) > 2 * EDGE then
        finish(me)
    end
end

-- 0x080DACF4: a hook at each panel around (x, y) holding what the reel's side catches: a hit there and the hook's
-- look, 20 ticks, around the reel's panel's center at its height; how many.
local function hook(me: Object, x: number, y: number): number
    local catches = CATCHES[me.alliance]
    if not catches then
        error(string.format("ElcReel's reel of side %d reads past 0x080DAD84", me.alliance))
    end
    local n = 0
    for _, h in HOOKS do
        local px, py = x + h.dx, y + h.dy
        if bit32.btest(field.flags(px, py), catches) then
            battle.hitbox(me, {
                panel_x = px,
                panel_y = py,
                element = me.element,
                z = 0,
                region = regions.single,
                hit_effect = sparks.elec,
                target = collision.hits_navis,
                self_type = collision.thrown,
                damage = me.damage,
                stamina = me.stamina,
                hit_mod = HIT_MOD,
            })
            n += 1
            local cx, cy = field.center(x, y)
            local e = battle.effect(Vec3.new(cx + h.dx * 20 * 0x10000, cy + h.dy * 12 * 0x10000, me.pos.z), h.effect)
            if e then
                e.timer = HOOK_LIFETIME
            end
        end
    end
    return n
end

-- 0x080DAC80: 5 ticks, then the hooks.
local function caught(me: Object)
    if me.phase_init == 0 then
        me.timer = WAIT
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t > 0 then
        return
    end
    me.timer2 = hook(me, me.panel_x, me.panel_y)
    me.action, me.phase, me.phase_init = SHOCK, 0, 0
end

-- 0x080DACAC: every 5 ticks (the first at once) its panel's shock, as many times as it hooked; then it is gone.
local function shock(me: Object)
    local c = me.collision
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t > 0 then
        return
    end
    me.timer = WAIT
    c:set_region(regions.single)
    local left = me.timer2 - 1
    me.timer2 = int.u16(left)
    if left < 0 then
        finish(me)
    end
end

-- 0x080DAC0E: the hits against it resolve and spark; its action; then present.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local a = me.action
    if a == ROLL then
        roll(me)
    elseif a == CAUGHT then
        caught(me)
    elseif a == SHOCK then
        shock(me)
    else
        error(string.format("ElcReel's reel's action %#x reads past 0x080DAC2C", a))
    end
    c:present()
end

-- 0x080DABE0: on its panel, its look and collision (covering its panel), the spark of its element; rolling forward.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "hidden"
    me.anim, me.anim_loaded = ANIM, 0xFF
    if not me:create_collision() then
        error("ElcReel's reel has no collision: 0x080DABE0 goes on through a null pointer")
    end
    me:setup_collision(collision.thrown, collision.hits_navis, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    me.collision:set_hit_effect(sparks.of_element(me.element))
    me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
    me.lifecycle = "update"
    tick(me)
end

elcreel.reel = define.kind {
    id = "elcreel/reel",
    pool = "attack",
    -- 0x080DABBC.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080DACDE and `sub_801155A`: a reel of `owner`'s on (x, y), with the element and the damage word, 26 pixels up.
-- (Its X and Y are the spawner's registers, the panel's Y and the element, until its init places it.)
local function spawn(owner: Object, x: number, y: number, element: number, damage: number): Object?
    local o = battle.spawn(elcreel.reel, Vec3.new(y, element, 0x1A_0000))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

export type ActionState = {
    -- The action's count (AIAttackVars+0x10).
    timer: number,
}

-- The reel launcher (attachment row 0x35: sheet 0c-38 at attach point 0x17), in animation 1.
local LAUNCHER = attachment.look { sprite = asset.sprite("reel-launcher"), palette = 0, attach_point = 0x17 }
local SOUND = asset.sound("erase-man-2")
local THROW_ANIM = 10
local LAUNCH_AT, LAUNCH_TICKS, RECOVER_TICKS = 1, 15, 5

-- The phases, on the attack's step.
local LAUNCH, RECOVER = 0x0, 0x4

-- 0x080ECE68: the launcher raised; on the count's 1 the reel; at 16 the recovery.
local function launching(me: Object, s: ActionState)
    if me.step_init == 0 then
        me:set_animation(THROW_ANIM)
        attachment.attach(me, LAUNCHER, "overlay", { anim = 1 })
        me:set_status("using_action", true)
        battle.play_sound(SOUND)
        me.step_init = 4
        s.timer = 0
    end
    if s.timer == LAUNCH_AT then
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, damage)
    end
    local t = s.timer + 1
    s.timer = t
    if t > LAUNCH_TICKS then
        me.step, me.step_init = RECOVER, 0
    end
end

-- 0x080ECED0: 6 ticks, then the launcher goes and back to idle.
local function recover(me: Object, s: ActionState)
    if me.step_init == 0 then
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me:set_status("using_action", false)
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end
end

elcreel.action = define.action {
    state = { timer = "u16" },
    -- 0x080ECE4C.
    update = function(me: Object, s: ActionState)
        local step = me.step
        if step == LAUNCH then
            launching(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("ElcReel's phase %#x reads past 0x080ECE60", step))
        end
    end,
}

return elcreel
