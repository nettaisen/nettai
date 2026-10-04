--!strict
-- The fan Wind and Fan place (the original's attack object #0x48,
-- `sub_80CCFDC`; docs/engine/dimming-chip-effects.md §1): a field obstacle
-- with 40 HP that stands for 1440 ticks and, from its second update, is its
-- side's wind and blows a gust (objects/gust) along the enemy's rows every
-- 12 ticks, rows 2, 1, 3 in turn: Wind's push away from the enemy's front
-- column, Fan's pull from the far column. What differs between the two is
-- the fan's variant (`byte_80CCFD4`, by the original's first parameter).
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions.

local absorbed_obstacle = require("../../objects/absorbed-obstacle/absorbed_obstacle")
local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local gust = require("../../objects/gust/gust")
local regions = require("@exelib/regions")

local fan = {}

export type State = {
    variant: FanVariant,
    -- `ExtraVars+0xC`: counts down to the next gust's row (3, 2, 1).
    row_counter: number,
}

local SPRITE = asset.sprite("bn6:fan")
-- How a fan looks absorbed (both variants' obstacle kind 0: its own
-- sprite).
fan.absorbed = absorbed_obstacle.look { sprite = SPRITE }
local LIFETIME = 0x5A0
-- Ticks between gusts.
local GUST_INTERVAL = 0xC
local SOUND_APPEAR = asset.sound("bn6:appear")
local SOUND_BLOW = asset.sound("bn6:dustbrk")
local SOUND_BREAK = asset.sound("bn6:hit-bomb-1")
-- What it reaches: its side's obstacles' (the other side's attacks,
-- objects and bodies, and neutral objects).
local IS, HITS, HIT_MOD = collision.own_body, collision.hit_by_other_side, 0
-- Where the gust blows from (`byte_80CD2A4`, by side): the fan's side's
-- own panels.
local OWN_PANELS: { [number]: { number } } = { [0] = { 0, 0x20 }, [1] = { 0x20, 0 } }
-- Another obstacle on the panel: no gust.
local NEUTRAL_OBJECT = 0x0080_0000

-- A fan's variant.
function fan.variant(v: FanVariant): FanVariant
    return define.record("fan-variant", v)
end

-- `sub_80CD26C`: Wind's gust starts on the enemy's first column of `row`:
-- in front of the fan's side's frontmost panel on the row (from the far
-- column back); none if another obstacle stands there. (With no panel of
-- its side on the row it starts a column past the field's edge.)
local function push_column(me: Object, row: number): number
    local back = bit32.bxor(me.alliance, me.flip)
    local step = if back == 0 then -1 else 1
    local rule = OWN_PANELS[me.alliance]
    if not rule then
        error(string.format("the fan's side %d reads past byte_80CD2A4", me.alliance))
    end
    local x = 6 - 5 * back
    local found = 0
    while field.valid(x, row) do
        if field.check(x, row, rule[1], rule[2]) then
            found = x
            break
        end
        x += step
    end
    x = found + me.facing
    if bit32.btest(field.flags(x, row), NEUTRAL_OBJECT) then
        return 0
    end
    return x
end

-- `sub_80CD236`: Fan's gust starts on the far column of `row`, or the
-- first panel toward the fan without another obstacle on it; none if that
-- leaves the field.
local function pull_column(me: Object, row: number): number
    local x = bit32.bxor(bit32.bxor(me.alliance, me.flip), 1) * 5 + 1
    while bit32.btest(field.flags(x, row), NEUTRAL_OBJECT) do
        x -= me.facing
        if x == 0 or x == 7 then
            return 0
        end
    end
    return x
end

-- Done: off the field, hidden, destroyed next update.
local function leave(me: Object)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80CD0E0`: placed: the sound, and on.
local function appear(me: Object)
    local v = (me.state :: State).variant
    battle.play_sound(SOUND_APPEAR)
    me.timer2 = v.wait
    me:set_action(8)
end

-- `sub_80CD0F8`: become the side's wind, then wait for the first gust.
local function rise(me: Object)
    if me.phase == 0 then
        battle.play_sound(SOUND_BLOW)
        battle.set_wind(me, me.alliance, "obstacle")
        me.phase = 4
        return
    end
    local t = me.timer2 - 1
    me.timer2 = t
    if t == 0 then
        (me.state :: State).row_counter = 3
        me:set_action(9)
    end
end

-- `sub_80CD144`: a gust every 12 ticks, the rows in turn; a row with no
-- panel to blow from is skipped at once. (It would hold while two bytes
-- that nothing sets were both nonzero.)
local function blow(me: Object)
    if me.phase == 0 then
        local s = me.state :: State
        local row = s.row_counter - 1
        if row == 0 then
            row = 3
        end
        s.row_counter = row
        local x = if s.variant.pulls then pull_column(me, row) else push_column(me, row)
        if x == 0 then
            return
        end
        -- The gust pulls with Fan's (its first parameter, 1 or 0).
        gust.spawn(x, row, me.alliance, me.flip, { pulls = s.variant.pulls }, 0, nil, 0)
        me.timer2 = GUST_INTERVAL
        me.phase = 4
        return
    end
    local t = me.timer2 - 1
    me.timer2 = t
    if t == 0 then
        me.phase = 0
    end
end

-- `sub_80CD1BE`: leave the field: broken (an explosion), removed by a chip
-- (a puff, or blinking out), or absorbed; no longer its side's wind.
local function destroyed(me: Object)
    local how = obstacle.removal(me)
    if how == "absorbed" then
        obstacle.fly_to_absorber(me, (me.state :: State).variant.absorbed)
    elseif how ~= "broken" then
        local blink = obstacle.blink_out(me)
        if blink == "blinking" then
            return
        elseif blink == "no" then
            local p = me.pos
            battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), effects.puff)
        end
    else
        battle.play_sound(SOUND_BREAK)
        battle.effect(me.pos, effects.explosion)
    end
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    battle.clear_wind(me)
    me.collision:set_region(nil)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    leave(me)
end

-- The fan's action table (`off_80CD0B8`): its own actions and the
-- framework's shared ones (a push knocks it back as far as the hit says).
local ACTIONS: { [number]: (Object) -> () } = {
    [0] = appear,
    [1] = obstacle.return_to_idle,
    [2] = destroyed,
    [3] = obstacle.flinch,
    [4] = obstacle.paralyzed,
    [5] = obstacle.knocked_back,
    [6] = obstacle.frozen,
    [7] = obstacle.bubbled,
    [8] = rise,
    [9] = blow,
}

-- `sub_80CD08C`: hits (a push keeps the damage), the lifetime, the
-- reaction, its action.
local function tick(me: Object)
    me.visible = true
    me.sprite.white = false
    obstacle.take_hits(me, "keeps_damage")
    obstacle.tick_lifetime(me)
    local a = obstacle.react(me)
    if a then
        local action = ACTIONS[a]
        if not action then
            error(string.format("fan action %d reads past its table", a))
        end
        action(me)
    end
    me:update_sprite()
    me.collision:present()
end

-- `sub_80CCFFC`: on its panel with its variant's look and HP; its
-- collision on its panel at once.
local function init(me: Object)
    local v = (me.state :: State).variant
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    -- sprite_noShadow: the first part is drawn with the sprite.
    sprite.shadow = "with_sprite"
    sprite.palette = v.palette
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:step()
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me.element = 0
    me.identity = v.identity
    me.hp, me.max_hp = v.hp, v.hp
    me.timer = LIFETIME
    -- (With no collision it is freed, its field-object registration left
    -- behind.)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(IS, HITS, HIT_MOD)
    me.collision:set_region(regions.single)
    me.collision:present()
    me.lifecycle = "update"
    tick(me)
end

fan.kind = define.kind {
    id = "bn6:wind/fan",
    pool = "attack",
    state = { variant = "record:fan-variant", row_counter = "u32" },
    -- `sub_80CCFDC`.
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

-- `sub_80CD2B4`: a fan of `variant` on (x, y), of `owner`'s side and
-- facing, running while dimmed, with the damage word `damage`; its side's
-- one field object of class 1 (a second replaces the first). Its position
-- is the spawner's registers until its init places it.
function fan.spawn(owner: Object, variant: FanVariant, x: number, y: number, damage: number): Object?
    local o = battle.spawn(fan.kind)
    if not o then
        return nil
    end
    (o.state :: State).variant = variant
    o:set_damage_word(damage)
    o.panel_x, o.panel_y = x, y
    o.alliance, o.flip = owner.alliance, owner.flip
    o.run_while_dimmed = true
    obstacle.register(o, o.alliance, 1)
    return o
end

return fan
