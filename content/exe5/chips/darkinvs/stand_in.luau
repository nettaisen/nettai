--!strict
-- DarkInvs's stand-in (EXE5's actor object #9, 0x080BA5C0; EXE5's own): drawn as the user (@exelib/dimming/stand_in's
-- look: its sprite, palette and parts, where the user is, the user vanishing) for 30 ticks, its palette flickering
-- between the user's and the dark one every other pair of ticks; then its parts off, the user back with a sound, and
-- its controller (./controller) goes on. A user that isn't a player (no chip reaches one) has none: the controller
-- goes on at once.

local stand_in = require("@exelib/dimming/stand_in")

-- MegaMan, by key (his navi's module reaches this one through his souls' Chaos Unison weapons).
local MEGAMAN = "megaman"

export type State = {
    -- The controller waiting for it (the flag it was given, +0x60: the controller's +0x0C).
    controller: Object?,
}
-- What the controller keeps (@exelib/dimming/stand_in's `Waiting`).
type Waiting = { busy: boolean }

local standin = {}

local SOUND = asset.sound("bug")
-- How long it flickers (to 0), and MegaMan's dark palette (0x0800DE82).
local FLICKER, DARK = 0x1E, 1

-- Its controller goes on (the flag it was given, cleared).
local function release(me: Object)
    local c = (me.state :: State).controller
    if c then
        (c.state :: Waiting).busy = false
    end
end

-- 0x0800DE82: the dark palette of the user's navi (MegaMan's 1; any other navi's dark chip fizzles before it).
local function dark_palette(me: Object): number
    if battle.navi(me.alliance).navi.id ~= MEGAMAN then
        error("DarkInvs's stand-in: only MegaMan's DarkInvs gets this far (0x0800DE82's link navi palette)")
    end
    return DARK
end

-- 0x080BA692: 30 ticks, the user's palette while bit 1 of the timer is set, else the dark one; then the user back.
local function tick(me: Object)
    local user = me.related1 :: Object
    if me.phase_init == 0 then
        me.timer = FLICKER
        me.phase_init = 4
    end
    me.sprite.palette = if bit32.btest(me.timer, 2) then user.sprite.palette else dark_palette(me)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    me:remove_parts_of(user)
    dimming.show_user(user)
    battle.play_sound(SOUND)
    release(me)
    me:set_lifecycle("destroy")
end

-- 0x080BA5E4: drawn as a player user (0x0800D1C0's actor type 2), the user hidden; else gone at once.
local function init(me: Object)
    local user = me.related1 :: Object
    if user.actor_type ~= "player" then
        release(me)
        me:free()
        return
    end
    stand_in.init(me, stand_in.overlay)
    me:set_lifecycle("update")
    tick(me)
end

standin.kind = new.kind {
    id = "darkinvs/stand-in",
    pool = "actor",
    state = { controller = "object" },
    -- 0x080BA5C0.
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
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080BA6F2: the stand-in for `user` on (x, y), of the user's side and facing, running while dimmed; `controller`
-- waits for it. (Its position is the spawner's registers until its init takes the user's.)
function standin.spawn(controller: Object, user: Object, x: number, y: number): Object?
    local o = battle.spawn(standin.kind)
    if o then
        o.panel_x, o.panel_y = x, y
        o.related1 = user
        local s = o.state :: State
        s.controller = controller
        ;(controller.state :: Waiting).busy = true
        o.alliance, o.flip = user.alliance, user.flip
        o.run_while_dimmed = true
    end
    return o
end

return standin
