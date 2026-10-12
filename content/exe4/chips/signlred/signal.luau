--!strict
-- SignlRed's signal (attack object #0x81, 0x080DD538: EXE4's own), a field obstacle of its side with 100 HP that
-- stands for 2400 ticks on the panel in front of its user. Three ticks after it comes it takes the panel if it is free
-- solid ground (else a puff, and it goes); then it cycles: red (animation 0) for 421 ticks, the other side's chips off
-- (BattleState +0x64's bit of that side, `sub_800138E`: `battle.set_chips_enabled`), then green (animation 1, the
-- BugFix glow's flash sound) for 51, that side's chips on again (`sub_8001382`). Broken, it explodes; however it goes
-- (broken, run out, or never standing), the other side's chips are on again (0x080DD73C). (Under event flag 0x1187,
-- which no netbattle sets, its green lights and its end would also hide draw task 17 of the HUD, the levels gauge's
-- sprite, on the other side's console: lib/main_started.)
--
-- An obstacle: its update runs the obstacle framework's steps (the `obstacle` service) around its own actions, with
-- EXE4's older ways: its action table (0x080DD5F8, 7 rows: row 1, 0x0801013A, also stops it running while paused;
-- row 3, 0x08010276, does nothing; row 6 its standing), a touch destroying it with its HP as it is (0x08013F3E), no
-- NameID, no tracking released.

local collision = require("../../rules/collision")

local signal = {}

local SPRITE = asset.sprite("signal")
-- It takes its panel; it turns green; it breaks.
local SOUND_STAND = asset.sound("tops")
local SOUND_GREEN = asset.sound("bugfix-flash")
local SOUND_BREAK = asset.sound("hit-bomb-1")
-- Effect rows 21 (a puff) and 0 (an explosion), 16 pixels over it.
local PUFF = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 }
local EXPLOSION = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }
local EFFECT_Z = 0x10_0000
local HP, LIFETIME, HIT_MOD = 100, 0x960, 3
-- Ticks before it takes its panel; of each light (to -1).
local APPEAR, RED, GREEN = 3, 0x1A4, 50
-- Where it can stand: a solid panel with nothing on it.
local SOLID, OCCUPIED = 0x10, 0x0F88_0080

-- The other side.
local function other_side(me: Object): number
    return bit32.bxor(me.alliance, 1)
end

-- 0x080DD71A: the other side's chips on again (0x080DD73C); out of the field-object registry, off the panels, its
-- panel's reservation dropped; hidden, and gone.
local function leave(me: Object)
    battle.set_chips_enabled(other_side(me), true)
    obstacle.unregister(me)
    me.collision:set_region(nil)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- Action 0 (0x080DD614): three ticks; then its panel if it is free solid ground (with a sound), else a puff and it
-- goes (0x080DD6EA).
local function appear(me: Object)
    local t = me.timer2 - 1
    me.timer2 = int.u16(t)
    if t ~= 0 then
        return
    end
    if not field.check(me.panel_x, me.panel_y, SOLID, OCCUPIED) then
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + EFFECT_Z), PUFF)
        leave(me)
        return
    end
    battle.play_sound(SOUND_STAND)
    me:set_action(obstacle.action_byte(me, 8))
end

-- Its standing (row 6, 0x080DD64C), by phase: red (0x080DD668), then green (0x080DD69A).
local function standing(me: Object)
    local phase = me.phase
    if phase == 0 then
        if me.phase_init == 0 then
            me.anim = 0
            battle.set_chips_enabled(other_side(me), false)
            me.timer2 = RED
            me.phase_init = 4
        end
        local t = me.timer2 - 1
        me.timer2 = int.u16(t)
        if t < 0 then
            me.phase, me.phase_init = 4, 0
        end
    elseif phase == 4 then
        if me.phase_init == 0 then
            me.anim = 1
            battle.play_sound(SOUND_GREEN)
            battle.set_chips_enabled(other_side(me), true)
            me.timer2 = GREEN
            me.phase_init = 4
        end
        local t = me.timer2 - 1
        me.timer2 = int.u16(t)
        if t < 0 then
            me.phase, me.phase_init = 0, 0
        end
    else
        error(string.format("SignlRed's signal: phase %#x reads past its table", phase))
    end
end

-- Action 2 (0x080DD700): an explosion and its sound; it leaves.
local function destroyed(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + EFFECT_Z), EXPLOSION)
    battle.play_sound(SOUND_BREAK)
    leave(me)
end

-- 0x0801013A: back to its idle, no longer running while paused.
local function return_to_idle(me: Object)
    me.run_while_paused = false
    obstacle.return_to_idle(me)
end

-- 0x080DD5F8, by the framework's numbers (its rows 0 to 6: 0, 1, 2, 3, 4, 5, 8).
local ACTIONS: { [number]: (Object) -> () } = {
    [0] = appear,
    [1] = return_to_idle,
    [2] = destroyed,
    [3] = function(_me: Object) end,
    [4] = obstacle.paralyzed,
    [5] = obstacle.knocked_back,
    [8] = standing,
}

-- 0x080DD5CC: hits (a push keeps the damage), the lifetime, the reaction (a crushing touch destroys it, its HP as it
-- is), its action; its sprite (a requested animation loaded, else stepped); present.
local function tick(me: Object)
    me.visible = true
    me.sprite.white = false
    obstacle.take_hits(me, "keeps_damage")
    obstacle.tick_lifetime(me)
    local a = obstacle.react(me, "destroys")
    if a then
        local action = ACTIONS[a]
        if not action then
            error(string.format("SignlRed's signal: action %d reads past its table", a))
        end
        action(me)
    end
    me:load_or_step_sprite()
    me.collision:present()
end

-- 0x080DD558: on its panel, facing its side's way, stepped once; 100 HP, its lifetime and its three ticks; its
-- collision (a body of its side, hit by the other side).
local function init(me: Object)
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
    me.hp, me.max_hp = HP, HP
    me.timer2 = APPEAR
    me.timer = LIFETIME
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.own_body, collision.hit_by_other_side, HIT_MOD)
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

signal.kind = new.kind {
    id = "signlred/signal",
    pool = "attack",
    -- 0x080DD538.
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

-- 0x080DD766: a signal on (x, y) with the damage word, of `owner`'s side (its alliance byte), running while dimmed;
-- its side's one field object of class 1. (Its position is the spawner's registers until its init places it.)
function signal.spawn(owner: Object, x: number, y: number, damage: number): Object?
    local o = battle.spawn(signal.kind)
    if not o then
        return nil
    end
    o:set_damage_word(damage)
    o.panel_x, o.panel_y = x, y
    o.alliance = owner.alliance
    o.run_while_dimmed = true
    obstacle.register(o, o.alliance, 1)
    return o
end

return signal
