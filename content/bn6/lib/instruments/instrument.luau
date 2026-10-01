--!strict
-- An instrument (the original's attack object #0x78, `sub_80D4088`;
-- docs/engine/dimming-chips.md §6.2): Fanfare's, Discord's, Timpani's and
-- Silence's. It stands in front of its user as one of the side's field
-- objects (an obstacle with 60 HP, for 2400 ticks), rests 30 ticks, then
-- plays: its tune every 64 ticks and its effect every third tick for the
-- playing time, then a pause; and again. What differs between the four is
-- the instrument record (`instrument.instrument { ... }`, a row of
-- `byte_80D4078` with the effect `off_80D4298` names), which each chip
-- defines.
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions.

local absorbed_obstacle = require("../../objects/absorbed-obstacle/absorbed_obstacle")
local collision = require("../../rules/collision")
local effects = require("../effects")

local instrument = {}

export type State = {
    instrument: Instrument,
    -- The ticks to the next effect and to the next tune (ExtraVars+4, +8).
    effect_in: number,
    tune_in: number,
}

local SPRITE = asset.sprite("instrument")
local HP = 60
local LIFETIME = 0x960
-- Its collision: another body of its side, reached by the other side.
local HIT_MOD = 3
-- How it looks absorbed (the original's obstacle kind's sprite: its own).
local ABSORBED = absorbed_obstacle.look { sprite = SPRITE }
local APPEAR_SOUND, BREAK_SOUND = asset.sound("appear"), asset.sound("hit-bomb-1")
-- Resting before it plays.
local REST_TICKS = 0x1E
-- The animations: standing, resting, playing.
local ANIM_STAND, ANIM_REST, ANIM_PLAY = 0, 3, 4
-- The effects' ticks between and the tunes' (the rows' bytes 1 and 3).
local EFFECT_PERIOD, TUNE_PERIOD = 3, 0x40

-- The actions (the object's action byte).
local APPEAR, DESTROYED, REST, PLAY = 0, 2, 8, 9

-- An instrument.
function instrument.instrument(i: Instrument): Instrument
    return define.record("instrument", i)
end

-- Gone from the field's bookkeeping, hidden, and destroyed at the next
-- update.
local function finish(me: Object)
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    me.collision:set_region(nil)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80D4198`: standing, it plays the appearing sound and rests.
local function appear(me: Object)
    me.anim = ANIM_STAND
    me.timer2 = 0xA
    battle.play_sound(APPEAR_SOUND)
    me:set_action(REST)
end

-- `sub_80D41B6` (`sub_80D41D8`, `sub_80D41E8`): rest 30 ticks, then play;
-- not while dimmed.
local function rest(me: Object)
    if battle.dimmed() then
        return
    end
    if me.phase == 0 then
        me.anim = ANIM_REST
        me.timer2 = REST_TICKS
        me.phase, me.phase_init = 4, 0
        return
    end
    local t = me.timer2 - 1
    me.timer2 = t
    if t == 0 then
        me:set_action(PLAY)
    end
end

-- `sub_80D41FC`: play (`sub_80D4228`, `sub_80D4242`), then pause
-- (`sub_80D42B0`, `sub_80D42CC`), and rest again; not while dimmed. The
-- battle over, it stops playing and stays paused.
local function play(me: Object)
    if battle.dimmed() then
        return
    end
    local s = me.state :: State
    local i = s.instrument
    local phase = me.phase
    if phase == 0 then
        me.anim = ANIM_PLAY
        me.timer2 = i.play_ticks
        s.effect_in, s.tune_in = 1, 1
        me.phase, me.phase_init = 4, 0
    elseif phase == 4 then
        if battle.over() then
            me.phase, me.phase_init = 8, 0
            return
        end
        s.tune_in -= 1
        if s.tune_in == 0 then
            s.tune_in = TUNE_PERIOD
            battle.play_sound(i.tune)
        end
        s.effect_in -= 1
        if s.effect_in == 0 then
            s.effect_in = EFFECT_PERIOD
            i.effect(me)
        end
        local t = me.timer2 - 1
        me.timer2 = t
        if t == 0 then
            me.phase, me.phase_init = 8, 0
        end
    elseif phase == 8 then
        me.anim = ANIM_STAND
        me.timer2 = i.pause_ticks
        me.phase, me.phase_init = 0xC, 0
    elseif phase == 0xC then
        if battle.over() then
            return
        end
        local t = me.timer2 - 1
        me.timer2 = t
        if t == 0 then
            me:set_action(REST)
        end
    else
        error(string.format("an instrument's playing phase %#x reads past off_80D4218", phase))
    end
end

-- `sub_80D42E8`: leaving: a chip's removal (absorbed, blinking out, or a
-- puff), else broken (an explosion and its sound).
local function destroyed(me: Object)
    local how = obstacle.removal(me)
    if how == "broken" then
        battle.play_sound(BREAK_SOUND)
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x14_0000), effects.explosion)
    elseif how == "absorbed" then
        obstacle.fly_to_absorber(me, ABSORBED)
    else
        local blink = obstacle.blink_out(me)
        if blink == "blinking" then
            return
        elseif blink == "no" then
            local p = me.pos
            battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), effects.puff)
        end
    end
    finish(me)
end

-- Its action table (`off_80D4170`): its own actions and the framework's
-- shared ones.
local ACTIONS: { [number]: (Object) -> () } = {
    [APPEAR] = appear,
    [1] = obstacle.return_to_idle,
    [DESTROYED] = destroyed,
    [3] = obstacle.flinch,
    [4] = obstacle.paralyzed,
    [5] = obstacle.knocked_back,
    [6] = obstacle.frozen,
    [7] = obstacle.bubbled,
    [REST] = rest,
    [PLAY] = play,
}

-- `sub_80D4144`.
local function tick(me: Object)
    me.visible = true
    me.sprite.white = false
    obstacle.take_hits(me, "keeps_damage")
    obstacle.tick_lifetime(me)
    local a = obstacle.react(me)
    if a then
        local action = ACTIONS[a]
        if not action then
            error(string.format("an instrument's action %d reads past off_80D4170", a))
        end
        action(me)
    end
    me:update_sprite()
    me.collision:present()
end

-- `sub_80D40A8`: on its panel, with its look, 60 HP and its lifetime.
local function init(me: Object)
    local i = (me.state :: State).instrument
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:step()
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me.name_id = i.identity
    me.hp, me.max_hp = HP, HP
    me.timer = LIFETIME
    sprite.palette = i.palette
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.own_body, collision.hit_by_other_side, HIT_MOD)
    me.collision:present()
    me.lifecycle = "update"
    tick(me)
end

instrument.kind = define.kind {
    id = "instrument",
    pool = "attack",
    state = { instrument = "record:instrument", effect_in = "u32", tune_in = "u32" },
    -- `sub_80D4088`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- `sub_80D4408`: `i` on panel (`x`, `y`), on `owner`'s side and facing,
-- with the damage word: one of the side's field objects (the side's
-- previous instrument, plane or turret breaks). It acts while dimmed.
function instrument.spawn(owner: Object, x: number, y: number, i: Instrument, damage: number): Object?
    local o = battle.spawn(instrument.kind, Vec3.zero)
    if not o then
        return nil
    end
    o:set_damage_word(damage)
    o.panel_x, o.panel_y = x, y
    o.alliance, o.flip = owner.alliance, owner.flip
    o.run_while_dimmed = true
    obstacle.register(o, o.alliance, 1)
    local s = o.state :: State
    s.instrument = i
    return o
end

return instrument
