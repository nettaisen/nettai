--!strict
-- CircGun's gun (the original's attack object #0x89, `sub_80D655C`;
-- docs/engine/dimming-chip-effects.md §8): a sight that circles the enemy's
-- area, a step every period + 1 ticks (down and up the far column, along
-- the rows, turning at the enemy's front and before a column wholly the
-- user's side's), until the user's side presses A or 360 ticks pass; then
-- each of its next steps leaves a shot (./shot) on the panel it leaves,
-- until its shots are out. It tells its controller once they are done.
-- Its sprite steps while dimmed. Another game's gun is a kind of its look
-- (`gun.make_kind`: ./shot's look, and its shots' kind).

local shot = require("./shot")

local gun = {}

export type State = {
    variant: CircGunVariant,
    -- The controller waiting on it (its flag, the original's Unk_0c).
    controller: Object?,
    -- The shots left to fire, 0xFF while aiming (the original's Unk_0c).
    shots_left: number,
    -- Shots fired (ExtraVars+0), and each one's flag while it is out
    -- (ExtraVars+4..).
    fired: number,
    shots_out: { number },
}

-- What it tells its controller.
type Waiting = { busy: boolean }

local ANIM = 1
local AIMING = 0xFF
local TIMEOUT, AFTER = 0x168, 0x1E
-- The panels whose column it won't step into when wholly theirs
-- (`byte_80D676C`, by side): the user's side's.
local OWN_PANELS: { [number]: { number } } = { [0] = { 0, 0x20 }, [1] = { 0x20, 0 } }

-- `sub_800D5F0`: how many panels of column `x` aren't `side`'s.
local function foreign_panels(x: number, side: number): number
    local n = 0
    for y = 1, 3 do
        local p = field.panel(x, y)
        if p and p.alliance ~= side then
            n += 1
        end
    end
    return n
end

-- A step's timing (both phases): the period on the phase's first tick; true
-- once it runs out.
local function step_due(me: Object, s: State): boolean
    if me.phase_init == 0 then
        me.timer = s.variant.period
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    return t < 0
end

-- `sub_80D6694`: up or down the column; at its end, turn to the rows.
local function vertical(me: Object, s: State, place: (Object) -> ())
    if not step_due(me, s) then
        return
    end
    local v = me.vel
    local y = me.panel_y + v.y
    if field.valid(me.panel_x, y) then
        me.panel_y = y
        place(me)
        me.phase_init = 0
        return
    end
    me.vel = Vec3.new(v.x, -v.y, v.z)
    me.timer = 0
    me.phase, me.phase_init = 4, 1
end

-- `sub_80D66E2`: along the row; turn back to the columns going back into a
-- column with none of the other side's panels, off the field, or before a
-- column wholly the user's side's.
local function horizontal(me: Object, s: State, place: (Object) -> ())
    if not step_due(me, s) then
        return
    end
    local v = me.vel
    local x = me.panel_x + v.x
    local turn = v.x ~= me.facing and foreign_panels(x, me.alliance) == 0
    if not turn then
        if not field.valid(x, me.panel_y) then
            turn = true
        else
            local rule = OWN_PANELS[me.alliance]
            local own = 0
            for y = 1, 3 do
                if field.check(x, y, rule[1], rule[2]) then
                    own += 1
                end
            end
            turn = own == 3
        end
    end
    if not turn then
        me.panel_x = x
        place(me)
        me.phase_init = 0
        return
    end
    me.vel = Vec3.new(-v.x, v.y, v.z)
    me.timer = 0
    me.phase, me.phase_init = 0, 1
end

-- A gun kind (`id`) of `look` (./shot's), firing shots of `shot_kind`.
function gun.make_kind(id: string, look: CircGunLook, shot_kind: Kind): Kind
    -- `sub_80D67D2`: on its panel, 8 pixels down and up (BN5's on it).
    local place = if look.on_floor then shot.place_on_floor else shot.place

    -- `sub_80D65FC`: aim until A (the side's player's, in the dimming's key
    -- record) or the timeout; step; firing, a shot where it left, until the
    -- last.
    local function circle(me: Object)
        local s = me.state :: State
        if s.shots_left == AIMING then
            local t = me.timer2 - 1
            me.timer2 = t
            local fire = t < 0
            if not fire then
                local player = battle.player(me.alliance)
                fire = player ~= nil and player:dimmed_pressed("a")
            end
            if fire then
                s.shots_left = s.variant.shots
            end
        end
        local ox, oy = me.panel_x, me.panel_y
        if me.phase == 0 then
            vertical(me, s, place)
        elseif me.phase == 4 then
            horizontal(me, s, place)
        else
            error(string.format("CircGun's gun: phase %#x reads past off_80D668C", me.phase))
        end
        if me.panel_x == ox and me.panel_y == oy then
            return
        end
        if s.shots_left == AIMING then
            battle.play_sound(look.sounds.step)
            return
        end
        s.shots_left -= 1
        if s.fired >= 8 then
            error("CircGun's gun: a ninth shot's flag lies past its variables")
        end
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        local ticks = s.variant.period * 4 + 0x14
        shot.spawn_kind(shot_kind, me, ox, oy, me.element, damage, ticks, s.variant.look, s.fired)
        s.fired += 1
        if s.shots_left == 0 then
            me.visible = false
            me:set_action(4)
        end
    end

    -- `sub_80D677C`: once its shots are done, 31 ticks; then its controller
    -- goes on (as the original, wherever the controller was).
    local function done(me: Object)
        local s = me.state :: State
        if me.phase_init == 0 then
            for i = 1, 4 do
                if s.shots_out[i] ~= 0 then
                    return
                end
            end
            me.timer = AFTER
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t >= 0 then
            return
        end
        local c = s.controller
        if c and c.active then
            (c.state :: Waiting).busy = false
        end
        me:set_lifecycle("destroy")
    end

    -- `sub_80D65E0`.
    local function tick(me: Object)
        local a = me.action
        if a == 0 then
            circle(me)
        elseif a == 4 then
            done(me)
        else
            error(string.format("CircGun's gun: action %#x reads past off_80D65F4", a))
        end
    end

    -- `sub_80D6580`: on its panel, aiming, stepping first toward the user and
    -- down. (The sprite starts on animation 0xFF, a held frame; animation 1
    -- loads at its first update.)
    local function init(me: Object)
        local s = me.state :: State
        local sprite = me.sprite
        sprite:load(look.sprite)
        -- sprite_noShadow: the first part is drawn with the sprite.
        sprite.shadow = "with_sprite"
        me.anim, me.anim_loaded = ANIM, 0xFF
        sprite:set_animation(0xFF)
        sprite:step()
        me.visible = true
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        place(me)
        sprite.palette = s.variant.look
        me.vel = Vec3.new(-me.facing, 1, me.vel.z)
        me.timer2 = TIMEOUT
        s.shots_left = AIMING
        me:set_lifecycle("update")
        tick(me)
    end

    return define.kind {
        id = id,
        pool = "attack",
        state = {
            variant = "record:circgun-variant",
            controller = "object",
            shots_left = "u8",
            fired = "u32",
            shots_out = "u8[8]",
        },
        -- `sub_80D655C`.
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
            me:update_sprite_while_dimmed()
        end,
    }
end

gun.kind = gun.make_kind("bn6:circgun/gun", shot.look, shot.kind)

-- `sub_80D67A6`: a gun of `variant` on (x, y) with `element` and the damage
-- word, of `controller`'s side and facing, running while dimmed;
-- `controller` waits until it is done. (Its position is the spawner's
-- registers until its init places it.)
function gun.spawn_kind(
    kind: Kind,
    controller: Object,
    variant: CircGunVariant,
    x: number,
    y: number,
    element: number,
    damage: number
): Object?
    local o = battle.spawn(kind)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o:set_damage_word(damage)
    o.alliance, o.flip = controller.alliance, controller.flip
    local s = o.state :: State
    s.variant, s.controller = variant, controller
    ;(controller.state :: Waiting).busy = true
    o.run_while_dimmed = true
    return o
end

-- BN6's gun.
function gun.spawn(controller: Object, variant: CircGunVariant, x: number, y: number, element: number, damage: number): Object?
    return gun.spawn_kind(gun.kind, controller, variant, x, y, element, damage)
end

return gun
