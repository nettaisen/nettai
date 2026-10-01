--!strict
-- The countdown bomb (attack object #0x4B, `sub_80CD8EC`) the TimeBom chips
-- set (objects/time-bom): a field obstacle that counts 3, 2, 1 and blows up
-- the whole enemy area (a hit over it and bursts over its panels:
-- objects/panel-bursts), unless it is broken first (a small puff, no
-- blast). Param1 is its variant (`VARIANTS`: 0 TimeBom1-3, 1 TimeBom+).
-- It stays until it goes off; it has no lifetime.
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions.

local absorbed_obstacle = require("../absorbed-obstacle/absorbed_obstacle")
local panel_bursts = require("../panel-bursts/panel_bursts")

local countdown_bomb = {}

local SPRITE = "0C-23"

-- `byte_80CD8AC`: the variants: the animations while appearing and
-- standing, HP, its field-object class (0: two a side, 1: one) and
-- collision types, and two behaviors: `bursts_when_broken` (broken, it
-- blows up; its countdown is only the last 15 ticks, after which it
-- blinks away) and `allows_bodies` (it may appear under a body). Only
-- variants 0 and 1 can be spawned (see `spawn`).
type Variant = {
    appear_anim: number,
    idle_anim: number,
    hp: number,
    bursts_when_broken: boolean,
    allows_bodies: boolean,
    class: number,
    self_type: number,
    target_type: number,
}
local VARIANTS: { [number]: Variant } = {
    [0] = { appear_anim = 0, idle_anim = 1, hp = 50, bursts_when_broken = false, allows_bodies = false, class = 1, self_type = 0xE, target_type = 0xF },
    [1] = { appear_anim = 2, idle_anim = 3, hp = 200, bursts_when_broken = false, allows_bodies = false, class = 1, self_type = 0xE, target_type = 0xF },
    [2] = { appear_anim = 4, idle_anim = 4, hp = 3, bursts_when_broken = false, allows_bodies = true, class = 0, self_type = 0xC, target_type = 0xD },
    [3] = { appear_anim = 5, idle_anim = 6, hp = 1, bursts_when_broken = true, allows_bodies = true, class = 0, self_type = 0xC, target_type = 0xD },
    [4] = { appear_anim = 4, idle_anim = 4, hp = 5, bursts_when_broken = false, allows_bodies = true, class = 0, self_type = 0xC, target_type = 0xD },
    [5] = { appear_anim = 5, idle_anim = 6, hp = 1, bursts_when_broken = true, allows_bodies = true, class = 0, self_type = 0xC, target_type = 0xD },
    [6] = { appear_anim = 4, idle_anim = 4, hp = 10, bursts_when_broken = false, allows_bodies = true, class = 0, self_type = 0xC, target_type = 0xD },
    [7] = { appear_anim = 5, idle_anim = 6, hp = 1, bursts_when_broken = true, allows_bodies = true, class = 0, self_type = 0xC, target_type = 0xD },
}

-- Its names by variant (TimeBom+ has its own).
local NAME, PLUS_NAME = 0xD8, 0xD9
local HIT_MOD = 3
local PLACE_SOUND = 0x94
-- Where it can stand once it is up: a solid panel on the field with
-- nothing on it (`allows_bodies`: bodies are fine).
local STANDS, OCCUPIED, OCCUPIED_BUT_BODIES = 0x10010, 0x0F88_0080, 0x0388_0080
-- The countdown: three steps of 60 ticks showing 3, 2 and 1 (the sprite
-- parts left hidden), then 30 more.
local STEP_TICKS, LAST_TICKS, SHORT_TICKS = 0x3C, 0x1E, 0xF
local SHOWS_3, SHOWS_2, SHOWS_1 = 0x3400_0000, 0x2C00_0000, 0x1C00_0000
local TICK_SOUND, LAST_SOUND = 0xC1, 0xC2
-- Blinking away: 8 ticks.
local BLINK_TICKS = 8
-- The blast (`byte_80CDD30`, by the side it hits for: the enemy area,
-- whole-field region 0x82 or 0x81), with bursts every 2 ticks (look 1).
local BLAST_REGION: { [number]: number } = { [0] = 0x82, [1] = 0x81 }
local BLAST_HIT_EFFECT, BLAST_TARGET, BLAST_SELF, BLAST_HIT_MOD = 0xFF, 5, 4, 3
local BURST_INTERVAL, BURST_LOOK = 2, 1
local BLAST_SOUND = 0xC3
-- A puff where it was broken (effect #0 look 0) and its sound; what's
-- left when a chip removes it or it can't stand (look 0x14, 12 or 8
-- pixels up).
local PUFF, PUFF_SOUND = 0, 0x70
local REMOVED_EFFECT = 0x14
-- How it looks absorbed (the original's obstacle kind 2): its own sprite,
-- with the sprite parts it hid (its digits); thrown, its parts keep their
-- own facing.
local ABSORBED = absorbed_obstacle.look {
    sprite = asset.sprite("countdown-bomb"),
    keeps_parts = true,
    own_facing = true,
}

-- Its actions (`off_80CD9F4`).
local APPEAR, DESTROYED, COUNTDOWN, BLINK_AWAY = 0, 2, 8, 9

local function variant(me: Object): Variant
    local v = VARIANTS[me:param(1)]
    if not v then
        error(string.format("countdown bomb variant %d reads past byte_80CD8AC", me:param(1)))
    end
    return v
end

-- `sub_80CDC7C`: a bomb for `side` on (x, y) with the spawn parameters
-- `params` (the variant first) and the damage word `damage`, registered in
-- the side's field-object slots. (Until its init places it, its position
-- is the spawner's registers: Y, the side and the bonus.)
function countdown_bomb.spawn(x: number, y: number, side: number, params: { number }, damage: number, bonus: number): Object?
    local o = battle.spawn_kind("countdown-bomb", Vec3.new(y, side, bonus), params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.alliance = side
    o.damage = bit32.band(damage, 0xFFFF)
    o.stamina = bit32.rshift(damage, 16)
    local v = variant(o)
    if o:param(1) > 1 then
        -- The game links these variants to a slot at the spawner's r7 (the
        -- bomb's related1; its init and end clear the slot), which its only
        -- spawner (`sub_80E3290`) leaves as whatever the register held.
        error(string.format("sub_80CDC7C: countdown bomb variant %d stores itself through a garbage pointer", o:param(1)))
    end
    o.run_while_dimmed = true
    obstacle.register(o, side, v.class)
    return o
end

-- `sub_80CDCBC`: blow up (not once the battle is over): a hit over the
-- enemy area, for the side opposite its panel's, bursts over it, a camera
-- shake (two for TimeBom+), the sound.
local function explode(me: Object)
    if battle.over() then
        return
    end
    me.visible = false
    me.collision.region = 0
    local panel = field.panel(me.panel_x, me.panel_y)
    if not panel then
        error("sub_80CDCBC reads the panel of a countdown bomb off the field")
    end
    me.alliance = bit32.bxor(panel.alliance, 1)
    local region = BLAST_REGION[me.alliance]
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = me.pos.z,
        region = region,
        hit_effect = BLAST_HIT_EFFECT,
        target = BLAST_TARGET,
        self_type = BLAST_SELF,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = BLAST_HIT_MOD,
    })
    panel_bursts.spawn(me, region, BURST_INTERVAL, 0, { BURST_LOOK })
    battle.shake_camera(1, 0x1E)
    battle.play_sound(BLAST_SOUND)
    if variant(me) == VARIANTS[1] then
        -- TimeBom+ shakes it harder.
        battle.shake_camera(3, 0x3C)
    end
end

-- Gone: hidden, out of the field-object slots, destroyed next update.
local function leave(me: Object)
    me.visible = false
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    me:set_lifecycle("destroy")
end

-- `sub_80CDA1C`: once the rising animation is over, stand if the panel
-- allows (the countdown starts), else go at once.
local function appear(me: Object)
    if not bit32.btest(me.sprite.frame_flags, 0x80) then
        return
    end
    local v = variant(me)
    local forbid = if v.allows_bodies then OCCUPIED_BUT_BODIES else OCCUPIED
    if not field.check(me.panel_x, me.panel_y, STANDS, forbid) then
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x8_0000), REMOVED_EFFECT)
        me:unreserve_panel(me.future_panel_x, me.future_panel_y)
        me.collision.region = 0
        leave(me)
        return
    end
    me.anim = v.idle_anim
    me.timer = STEP_TICKS
    me.collision.region = 1
    me.action, me.phase, me.phase_init = COUNTDOWN, 0, 0
    if me:param(1) > 1 then
        me:reserve_panel(me.future_panel_x, me.future_panel_y)
    end
    if v.bursts_when_broken then
        me.phase = 8
        me.timer = SHORT_TICKS
    end
end

-- One countdown step: when the timer runs out, show the next number.
local function count(me: Object, shows: number, ticks: number, sound: number?, next_phase: number)
    local t = me.timer - 1
    me.timer = t
    if t ~= 0 then
        return
    end
    me.sprite.hidden_parts = shows
    me.timer = ticks
    if sound then
        battle.play_sound(sound)
    end
    me.phase = next_phase
end

-- `sub_80CDAD8` (phases `sub_80CDAFC`, `sub_80CDB2C`, `sub_80CDB50`,
-- `sub_80CDB80`): 3, 2, 1, then blow up (or blink away).
local function countdown(me: Object)
    local v = variant(me)
    local phase = me.phase
    if phase == 0 then
        -- (sub_80CDAFC tests for its first tick of TimeBom and TimeBom+
        -- and does nothing about it.)
        count(me, SHOWS_3, STEP_TICKS, TICK_SOUND, 4)
    elseif phase == 4 then
        count(me, SHOWS_2, STEP_TICKS, TICK_SOUND, 8)
    elseif phase == 8 then
        count(me, SHOWS_1, LAST_TICKS, if v.bursts_when_broken then nil else LAST_SOUND, 0xC)
    elseif phase == 0xC then
        local t = me.timer - 1
        me.timer = t
        if t ~= 0 then
            return
        end
        if v.bursts_when_broken then
            me.action, me.phase, me.phase_init = BLINK_AWAY, 0, 0
        else
            explode(me)
            me.action, me.phase, me.phase_init = DESTROYED, 0, 0
        end
    else
        error(string.format("the countdown bomb's countdown phase %#x reads past its table", phase))
    end
end

-- `sub_80CDBAC`: blink for 8 ticks, then go.
local function blink_away(me: Object)
    if me.phase == 0 then
        me.phase = 1
        me.collision.region = 0
        me.timer = BLINK_TICKS
    end
    me.visible = bit32.btest(me.timer, 2)
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        me.action, me.phase, me.phase_init = DESTROYED, 0, 0
    end
end

-- `sub_80CDBE8`: leave the field: a puff when broken (a blast for
-- `bursts_when_broken` variants, once the battle isn't dimmed), or as a
-- chip removed it.
local function destroyed(me: Object)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    me.collision.region = 0
    local how = obstacle.removal(me)
    if how == "broken" then
        if me.hp == 0 and variant(me).bursts_when_broken then
            if battle.dimmed() then
                return
            end
            explode(me)
        else
            battle.play_sound(PUFF_SOUND)
            battle.effect(me.pos, PUFF)
        end
    elseif how == "absorbed" then
        obstacle.fly_to_absorber(me, ABSORBED)
    else
        local blink = obstacle.blink_out(me)
        if blink == "blinking" then
            return
        elseif blink == "no" then
            local p = me.pos
            battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), REMOVED_EFFECT)
        end
    end
    leave(me)
end

-- The bomb's action table (`off_80CD9F4`): its own actions and the
-- framework's shared ones.
local ACTIONS: { [number]: (Object) -> () } = {
    [0] = appear,
    [1] = obstacle.return_to_idle,
    [2] = destroyed,
    [3] = obstacle.flinch,
    [4] = obstacle.paralyzed,
    [5] = obstacle.knocked_back,
    [6] = obstacle.frozen,
    [7] = obstacle.bubbled,
    [8] = countdown,
    [9] = blink_away,
}

-- `sub_80CD9CA`: at the end of the battle its HP drops to 0 (so it's
-- broken); hits (a push keeps the damage), the reaction, its action.
local function tick(me: Object)
    me.sprite.white = false
    if battle.over() then
        me.hp = 0
    end
    obstacle.take_hits(me, "keeps_damage")
    local a = obstacle.react(me)
    if a then
        local action = ACTIONS[a]
        if not action then
            error(string.format("countdown bomb action %d reads past its table", a))
        end
        action(me)
    end
    me:update_sprite()
    me.collision:present()
end

-- `sub_80CD91E`: on its panel, rising, with the variant's HP and a sound.
local function init(me: Object)
    local v = variant(me)
    me:set_coordinates_from_panel()
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim, me.anim_loaded = v.appear_anim, v.appear_anim
    sprite:set_animation(v.appear_anim)
    sprite.shadow = "ground"
    -- (sub_8002EAC sets a drawing flag of the sprite's.)
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    battle.play_sound(PLACE_SOUND)
    me.name_id = if me:param(1) == 1 then PLUS_NAME else NAME
    me.hp, me.max_hp = v.hp, v.hp
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(v.self_type, v.target_type, HIT_MOD)
    me.collision.region = 0
    me.collision:present()
    me.lifecycle = "update"
    tick(me)
end

-- `sub_80CD8EC`.
function countdown_bomb.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return countdown_bomb
