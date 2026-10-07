--!strict
-- A guard's shield, as EXE5 and EXE6 share it (EXE6's Reflectors' attack object #0x2B, `sub_80C96A0`; EXE5's guard's
-- #0x2B, 0x080CA22C, the same code; EXE6's chips.md §3.3): the barrier a navi holds up while it guards. Visual only:
-- the guard is the navi's own status. It follows its owner at the owner's attach point 6, shows while its owner does,
-- and ends with a short fade once the owner clears the slot it was stored in (its first related object). It runs
-- while paused and dimmed, but doesn't animate while dimmed. Each game makes its kind (`shield.make`) with its sound;
-- its looks are the game's rows (`shield.look`). EXE4's guard's (#0x2B, 0x080D2224) is the same code but for what
-- its `make` states besides: running while dimmed alone, animating then too, no test of its owner's having
-- vanished, and its spawn taking its owner's side alone.

local slot = require("@exelib/slot")

local shield = {}

-- A shield's look (`ReflectorShieldLook`, EXE6's types.d.luau; a row of EXE6's `byte_80C9664`, by the original's
-- second parameter).
export type Look = ReflectorShieldLook

export type State = {
    slot: Slot,
    look: Look,
}

export type Shield = {
    kind: Kind,
    -- `sub_80C97E0`: a shield of `look` for `owner`, `offset` from it.
    spawn: (owner: Object, offset: Vec3, l: Look) -> Object?,
}

function shield.look(l: Look): Look
    return new.record("reflector-shield-look", l)
end

-- The actions: held up, then fading.
local UP, FADING = 0, 4

-- `sub_80C9774`: at the owner's position plus the offset kept in the velocity, shown as the owner is; once the slot
-- is cleared, fade.
local function follow(me: Object)
    local owner = me.related1 :: Object
    me.pos = owner.pos + me.vel
    me:copy_visibility(owner)
    if not slot.occupied(owner, (me.state :: State).slot) then
        me.action = FADING
        me.phase = 0
        me.phase_init = 0
    end
end

-- `sub_80C97B4`: play the fade, then go.
local function fade(me: Object)
    if me.phase_init == 0 then
        me.visible = true
        local l = (me.state :: State).look
        me.anim = l.fade_anim
        me.timer = l.fade_ticks
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me:set_lifecycle("destroy")
    end
end

-- A game's shield kind: its key and sound; `animates_while_dimmed` (EXE4's 0x080D2224 steps its sprite whatever
-- the dimming), `runs_while_paused` (none: true; EXE4's spawn sets the dimming's bit alone, 0x080D2338),
-- `hides_with_vanished_owner` (none: true; EXE4's tick, 0x080D2298, tests nothing of the kind) and `side_only` (its
-- spawn takes its owner's side alone, not its facing: EXE4's).
function shield.make(spec: {
    id: string,
    sound: Sound,
    animates_while_dimmed: boolean?,
    runs_while_paused: boolean?,
    hides_with_vanished_owner: boolean?,
    side_only: boolean?,
}): Shield
    local hides_with_owner = spec.hides_with_vanished_owner ~= false
    local paused_too = spec.runs_while_paused ~= false
    -- `sub_80C9726`: the action, then shown unless the viewer is blind to it or its owner vanished for a navi chip.
    local function tick(me: Object)
        local a = me.action
        if a == UP then
            follow(me)
        elseif a == FADING then
            fade(me)
        else
            error(string.format("reflector shield action %#x reads past its table", a))
        end
        me.visible = true
        me:hide_from_blind()
        if hides_with_owner then
            local owner = me.related1 :: Object
            if owner:navi_state("vanished") then
                me.visible = false
            end
        end
    end

    -- `sub_80C96C8`: the look, the panel under the offset (the spawn position), and the offset kept in the velocity.
    local function init(me: Object)
        local l = (me.state :: State).look
        local sprite = me.sprite
        sprite:load(l.sprite)
        sprite.shadow = "with_sprite"
        me.visible = true
        me.anim = l.anim
        me.anim_loaded = 0xFF
        sprite.palette = l.palette
        me:set_panel_from_coordinates()
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        battle.play_sound(spec.sound)
        me.vel = me.pos
        me:set_lifecycle("update")
        tick(me)
    end

    local kind = new.kind {
        id = spec.id,
        pool = "attack",
        state = { slot = slot.TYPE, look = "record:reflector-shield-look" },
        -- `sub_80C96A0`: the state, then the sprite unless dimmed.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                -- object_freeMemory
                me:free()
                return
            end
            if spec.animates_while_dimmed or not battle.dimmed() then
                me:update_sprite()
            end
        end,
    }

    -- `sub_80C97E0`: a shield of `look` for `owner`, `offset` from it (its attach point, in the spawn position);
    -- stored in the owner's first related object, unless the pool is full.
    local function spawn(owner: Object, offset: Vec3, l: Look): Object?
        local o = battle.spawn(kind, offset)
        if o then
            o.related1 = owner
            local s = o.state :: State
            s.slot, s.look = "related", l
            slot.fill(owner, "related", o)
            o.alliance = owner.alliance
            if not spec.side_only then
                o.flip = owner.flip
            end
            o.run_while_paused = paused_too
            o.run_while_dimmed = true
        end
        return o
    end

    return { kind = kind, spawn = spawn }
end

return shield
