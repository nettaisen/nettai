--!strict
-- A Reflector's shield (the original's attack object #0x2B, `sub_80C96A0`):
-- the barrier a navi holds up while it guards. Visual only: the guard is
-- the navi's own status. It follows its owner at the owner's attach point
-- 6, shows while its owner does, and ends with a short fade once the owner
-- clears the slot it was stored in (its first related object). It runs
-- while paused and dimmed, but doesn't animate while dimmed. See chips.md
-- §3.3.

local slot = require("../../lib/slot")

local shield = {}

-- A shield's look (`ReflectorShieldLook`, types.d.luau; a row of
-- `byte_80C9664`, by the original's second parameter).
export type Look = ReflectorShieldLook

export type State = {
    slot: Slot,
    look: Look,
}

local function look(l: Look): Look
    return define.record("reflector-shield-look", l)
end

-- `byte_80C9664`'s rows: Rflectr1's, the NaviCust Reflect's and Shield's,
-- four no chip or program ported holds up, Rflectr2's and Rflectr3's, and
-- one more nothing ported holds up.
local REFLECTOR, NAVI, SHIELD = asset.sprite("reflector-shield"), asset.sprite("megaman-navi"), asset.sprite("dummy-shield")
local ROWS: { [number]: Look } = {
    [0] = look { sprite = REFLECTOR, anim = 0, fade_anim = 1, fade_ticks = 0xE, palette = 0 },
    [1] = look { sprite = REFLECTOR, anim = 2, fade_anim = 3, fade_ticks = 0xE, palette = 0 },
    [2] = look { sprite = REFLECTOR, anim = 4, fade_anim = 5, fade_ticks = 0xE, palette = 0 },
    [3] = look { sprite = NAVI, anim = 3, fade_anim = 4, fade_ticks = 4, palette = 7 },
    [4] = look { sprite = SHIELD, anim = 3, fade_anim = 4, fade_ticks = 1, palette = 0 },
    [5] = look { sprite = SHIELD, anim = 3, fade_anim = 4, fade_ticks = 1, palette = 2 },
    [6] = look { sprite = SHIELD, anim = 3, fade_anim = 4, fade_ticks = 1, palette = 4 },
    [7] = look { sprite = REFLECTOR, anim = 6, fade_anim = 7, fade_ticks = 0xE, palette = 0 },
    [8] = look { sprite = REFLECTOR, anim = 8, fade_anim = 9, fade_ticks = 0xE, palette = 0 },
    [9] = look { sprite = asset.sprite("sword"), anim = 0xA, fade_anim = 0xB, fade_ticks = 0xE, palette = 0 },
}
-- The looks by who holds them up: the chips, and the NaviCust programs.
shield.looks = {
    rflectr1 = ROWS[0],
    rflectr2 = ROWS[7],
    rflectr3 = ROWS[8],
    navicust_reflect = ROWS[1],
    navicust_shield = ROWS[2],
}

local SOUND = asset.sound("follow-effect")

-- The actions: held up, then fading.
local UP, FADING = 0, 4

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
    battle.play_sound(SOUND)
    me.vel = me.pos
    me:set_lifecycle("update")
    tick(me)
end

shield.kind = define.kind {
    id = "rflectr/shield",
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
        if not battle.dimmed() then
            me:update_sprite()
        end
    end,
}

-- `sub_80C97E0`: a shield of `look` for `owner`, `offset` from it (its
-- attach point, in the spawn position); stored in the owner's first
-- related object, unless the pool is full.
function shield.spawn(owner: Object, offset: Vec3, l: Look): Object?
    local o = battle.spawn(shield.kind, offset)
    if o then
        o.related1 = owner
        local s = o.state :: State
        s.slot, s.look = "related", l
        slot.fill(owner, "related", o)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_paused = true
        o.run_while_dimmed = true
    end
    return o
end

return shield
