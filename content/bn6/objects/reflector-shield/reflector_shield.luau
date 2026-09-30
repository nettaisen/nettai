--!strict
-- A Reflector's shield (attack object #0x2B, `sub_80C96A0`): the barrier
-- a navi holds up while it guards. Visual only: the guard is the navi's
-- own status. It follows its owner at the owner's attach point 6, shows
-- while its owner does, and ends with a short fade once the owner clears
-- the slot it was stored in (its first related object). It runs while
-- paused and dimmed, but doesn't animate while dimmed. See chips.md §3.3.
--
-- Its look (the second parameter) picks a row of `byte_80C9664`.

local slot = require("../../lib/slot")

export type State = {
    slot: Slot,
}

local reflector_shield = {
    state = { slot = slot.TYPE },
}

-- `byte_80C9664`, by the second parameter: the sprite, its animation
-- while up, the animation and ticks of its fade, and its palette.
type Look = { sprite: SpriteId, anim: number, fade_anim: number, fade_ticks: number, palette: number }
local LOOKS: { [number]: Look } = {
    [0] = { sprite = "0c-1b", anim = 0, fade_anim = 1, fade_ticks = 0xE, palette = 0 },
    { sprite = "0c-1b", anim = 2, fade_anim = 3, fade_ticks = 0xE, palette = 0 },
    { sprite = "0c-1b", anim = 4, fade_anim = 5, fade_ticks = 0xE, palette = 0 },
    { sprite = "08-00", anim = 3, fade_anim = 4, fade_ticks = 4, palette = 7 },
    { sprite = "04-00", anim = 3, fade_anim = 4, fade_ticks = 1, palette = 0 },
    { sprite = "04-00", anim = 3, fade_anim = 4, fade_ticks = 1, palette = 2 },
    { sprite = "04-00", anim = 3, fade_anim = 4, fade_ticks = 1, palette = 4 },
    { sprite = "0c-1b", anim = 6, fade_anim = 7, fade_ticks = 0xE, palette = 0 },
    { sprite = "0c-1b", anim = 8, fade_anim = 9, fade_ticks = 0xE, palette = 0 },
    { sprite = "0c-00", anim = 0xA, fade_anim = 0xB, fade_ticks = 0xE, palette = 0 },
}

local SOUND = 0xA0

-- The actions: held up, then fading.
local UP, FADING = 0, 4

-- `sub_80C97E0`: a shield for `owner`, `offset` from it (its attach point,
-- in the spawn position), with the chip's parameters; stored in the
-- owner's first related object, unless the pool is full.
function reflector_shield.spawn(owner: Object, offset: Vec3, params: { number }): Object?
    local o = battle.spawn_kind("reflector-shield", offset, params)
    if o then
        o.related1 = owner
        local s = o.state :: State
        s.slot = "related"
        slot.fill(owner, "related", o)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_paused = true
        o.run_while_dimmed = true
    end
    return o
end

-- The shield's row of `byte_80C9664`.
local function look(me: Object): Look
    local l = LOOKS[me:param(2)]
    if not l then
        error(string.format("reflector shield look %d reads past byte_80C9664", me:param(2)))
    end
    return l
end

-- `sub_800EB6C`: the local player sees the other side's things unless
-- their navi is blind.
local function viewer_sees(alliance: number): boolean
    local viewer = battle.local_side()
    if alliance == viewer then
        return true
    end
    local p = battle.player(viewer)
    return not (p and p:status("blind"))
end

-- `sub_80C9774`: at the owner's position plus the offset kept in the
-- velocity, shown as the owner is; once the slot is cleared, fade.
local function follow(me: Object)
    local owner = me.related1 :: Object
    me.pos = owner.pos + me.vel
    me.visible = owner.visible
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
        local l = look(me)
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

-- `sub_80C9726`: the action, then shown unless the local player is blind
-- to it or its owner vanished for a navi chip.
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
    if not viewer_sees(me.alliance) then
        me.visible = false
    end
    local owner = me.related1 :: Object
    if owner:navi_state("vanished") then
        me.visible = false
    end
end

-- `sub_80C96C8`: the look, the panel under the offset (the spawn
-- position), and the offset kept in the velocity.
local function init(me: Object)
    local l = look(me)
    local sprite = me.sprite
    sprite:load(l.sprite)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = l.anim
    me.anim_loaded = 0xFF
    sprite.palette = l.palette
    me:set_panel_from_coordinates()
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    battle.play_sound(SOUND)
    me.vel = me.pos
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80C96A0`: the state, then the sprite unless dimmed.
function reflector_shield.update(me: Object)
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
    if not battle.dimmed() then
        me:update_sprite()
    end
end

return reflector_shield
