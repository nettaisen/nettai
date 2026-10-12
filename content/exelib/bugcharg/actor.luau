--!strict
-- BugCharg's stand-in (actor object #0x59: EXE4's 0x080CC72C, EXE5's 0x080C3A40, on the dimming stand-ins' skeleton:
-- @exelib/dimming/stand_in), made for a game's look (`actor.make`): drawn as the user, it raises its charger (an
-- attachment in its `held`) with a sound and counts its shots, and 41 ticks on fires: a heavy shot (the game's flying
-- shot) every 16 ticks from the panel ahead, 20 pixels up, each with a sound and the camera shaken. 31 ticks after the
-- last it gives the user back, and its controller (@exelib/bugcharg/controller) moves on.
--
-- The games' differ in how many shots: EXE4's as many as the custom screens so far, at most 10; EXE5's one, and one
-- more for each NaviCust bug of its user's it takes in. And in their looks: what the stand-in takes and wears of its
-- user, its animations, its charger, its sounds, its shot, and whether it hides from a viewer blind to its side.

local stand_in = require("@exelib/dimming/stand_in")

local actor = {}

export type State = {
    -- The controller waiting on it (@exelib/dimming/stand_in).
    controller: Object?,
    -- Its charger (+0x64).
    held: Object?,
}

export type Look = {
    -- The kind's id.
    id: string,
    -- What it takes of its user as it is sent out, past its side (EXE5's its facing and NameID, which its charger's
    -- attach point reads; EXE4's nothing).
    adopt: (me: Object, user: Object) -> (),
    -- What it wears of its user (`stand_in.init`'s), and takes off as it leaves.
    wear: (me: Object, user: Object) -> (),
    unwear: (me: Object, user: Object) -> (),
    -- Each tick it is shown, but to a viewer blind to its side (EXE5's; EXE4's stays as its init left it).
    hides_from_blind: boolean,
    -- Its animation standing and raising its charger (EXE5's 16 and 10, EXE4's 10 and 13).
    stand_anim: number,
    raise_anim: number,
    -- Its charger raised: the game's attachment, into its `held`.
    raise: (me: Object) -> (),
    -- Its sounds: the charger raised, a shot.
    raise_sound: Sound,
    shot_sound: Sound,
    -- Its shots, counted as it raises its charger.
    shots: (me: Object) -> number,
    -- A shot from `pos` with the damage word (the game's flying shot's heavy row, in the charger's sprite).
    launch: (me: Object, pos: Vec3, damage: number) -> Object?,
}

export type Actor = {
    kind: Kind,
    -- The stand-in for `user` on `controller`'s panel (EXE4's 0x080CC90C, EXE5's 0x080C3C3A): its element the
    -- controller's side byte, the damage word `damage`, of the user's side; `controller` is busy until it leaves.
    spawn: (controller: Object, user: Object, damage: number) -> Object?,
}

-- What a controller waiting on it keeps: it is out (+0x0C).
type Waiting = { busy: boolean }

local AIM, BETWEEN, AFTER = 0x28, 0xF, 0x1E
-- A shot's height, in pixels (EXE4's by its navi, 0x080CC8BC: every one's 20).
local SHOT_Z = 0x14_0000

function actor.make(look: Look): Actor
    -- Phase 0 (EXE4's 0x080CC7F4, EXE5's 0x080C3B50): its charger raised (a sound), its shots counted; 41 ticks; then
    -- the charger's next animation.
    local function aim(me: Object)
        if me.phase_init == 0 then
            me.anim = look.raise_anim
            look.raise(me)
            battle.play_sound(look.raise_sound)
            me.timer = AIM
            me.timer2 = look.shots(me)
            me.phase_init = 4
        end
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left >= 0 then
            return
        end
        local held = (me.state :: State).held
        if held then
            held.anim += 1
        end
        me.timer = 0
        me.phase, me.phase_init = 4, 0
    end

    -- Phase 4 (EXE4's 0x080CC85C, EXE5's 0x080C3BA6): a shot every 16 ticks (a sound, the camera shaken); after the
    -- last, the charger's animation back.
    local function fire(me: Object)
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left >= 0 then
            return
        end
        me.timer = BETWEEN
        local x, y = field.center(me.panel_x + me.facing, me.panel_y)
        local shot = look.launch(me, Vec3.new(x, y, SHOT_Z), bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
        if shot then
            shot.run_while_dimmed = true
        end
        battle.play_sound(look.shot_sound)
        battle.shake_camera(2, 0x14)
        local n = me.timer2 - 1
        me.timer2 = int.u16(n)
        if n <= 0 then
            local held = (me.state :: State).held
            if held then
                held.anim -= 1
            end
            me.phase, me.phase_init = 8, 0
        end
    end

    -- Phase 8 (EXE4's 0x080CC8CC, EXE5's 0x080C3BF6): 31 ticks; then the charger let go, what it wears off, hidden and
    -- freed at its next update, the user back, and the controller on.
    local function finish(me: Object)
        if me.phase_init == 0 then
            me.timer = AFTER
            me.phase_init = 4
        end
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left >= 0 then
            return
        end
        local s = me.state :: State
        s.held = nil
        local user = me.related1 :: Object
        look.unwear(me, user)
        me.visible = false
        me:set_lifecycle("destroy")
        dimming.show_user(user)
        local c = s.controller
        if c and c.active then
            (c.state :: Waiting).busy = false
        end
    end

    -- EXE4's 0x080CC7D4, EXE5's 0x080C3B16: by phase.
    local function tick(me: Object)
        if look.hides_from_blind then
            stand_in.show(me)
        end
        local phase = me.phase
        if phase == 0 then
            aim(me)
        elseif phase == 4 then
            fire(me)
        elseif phase == 8 then
            finish(me)
        else
            error(string.format("BugCharg's stand-in: phase %#x reads past its table", phase))
        end
    end

    -- EXE4's 0x080CC750, EXE5's 0x080C3A64: drawn as the user (stand_in.init), standing; its first update.
    local function init(me: Object)
        stand_in.init(me, look.wear)
        me.anim = look.stand_anim
        me:set_lifecycle("update")
        tick(me)
    end

    local kind = new.kind {
        id = look.id,
        pool = "actor",
        state = { controller = "object", held = "object" },
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                me:free()
                return
            end
            me:update_sprite()
        end,
    }

    local function spawn(controller: Object, user: Object, damage: number): Object?
        local o = battle.spawn(kind)
        if o then
            o.panel_x, o.panel_y = controller.panel_x, controller.panel_y
            o.element = controller.alliance
            o:set_damage_word(damage)
            o.related1 = user;
            (o.state :: State).controller = controller;
            (controller.state :: Waiting).busy = true
            o.alliance = user.alliance
            look.adopt(o, user)
            o.run_while_dimmed = true
        end
        return o
    end

    return { kind = kind, spawn = spawn }
end

return actor
