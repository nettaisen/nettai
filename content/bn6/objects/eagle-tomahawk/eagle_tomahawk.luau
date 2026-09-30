--!strict
-- TomahawkMan's axe (effect object #0x32, `sub_80E3FD0`): the tomahawk
-- he swings for ETomahwk, riding on him, wearing his header flags and
-- look. Its action 1 (set by the chip's routine) swings it (animation 7),
-- 2 ends it; so do the battle's end and its owner leaving the action it
-- belongs to (by Param1: 0xC, or 0xA for the link chip's). Ending, it
-- clears the slot where its owner keeps it. [unverified: no lab scenario
-- reaches it; docs/engine/standard-chips.md]

local eagle_tomahawk = {}

local SPRITE = "10-2a"
local RAISED, SWUNG = 6, 7
-- The owner's action it lasts through, by Param1 (`byte_80E40C0`).
local OWNER_ACTIONS = { [0] = 0xC, 0xA }

-- `sub_80E40C2`: the axe on `owner`, which keeps it (the game passes the
-- slot, the owner's attack variables +0x30: the link chip's `held`).
function eagle_tomahawk.spawn(owner: Object, kind: number): Object?
    local o = battle.spawn_kind("eagle-tomahawk", owner.pos, { kind })
    if o then
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

-- `*ExtraVars = 0`: clear the owner's slot for it. The game writes the
-- attack variables' word +0x30 whatever the owner is running: the link
-- chip's `held` while it runs the chip, else the engine's marker.
local function clear_slot(owner: Object)
    if owner.action == 0xA then
        (owner.attack_state :: LinkChipState).held = nil
    else
        owner.marker = 0
    end
end

-- `sub_80E403E`.
local function tick(me: Object)
    local owner = me.related1 :: Object
    me.pos = owner.pos
    me.visible = owner.visible
    me.run_while_paused = owner.run_while_paused
    me.no_sprite_update = owner.no_sprite_update
    me.run_while_dimmed = owner.run_while_dimmed
    me.holds_reservation = owner.holds_reservation
    local look, worn = me.sprite, owner.sprite
    look.color_shader = worn.color_shader
    look.white = worn.white
    look.hflip = worn.hflip
    look.vflip = worn.vflip
    look.alpha = worn.alpha
    local lasts = OWNER_ACTIONS[me:param(1)]
    if lasts == nil then
        error(string.format("eagle tomahawk kind %d reads past byte_80E40C0", me:param(1)))
    end
    if not battle.over() and owner.action == lasts then
        local a = me.action
        if a == 1 then
            me.anim = SWUNG
            me.anim_loaded = 0xFF
            me.action = 0
            return
        elseif a ~= 2 then
            return
        end
    end
    clear_slot(owner)
    me:set_lifecycle("destroy")
end

-- `sub_80E3FFC`.
local function init(me: Object)
    me:set_panel_from_coordinates()
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite:set_animation(0)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = RAISED
    me.anim_loaded = RAISED
    sprite:set_animation(RAISED)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- `sub_80E3FD0`.
function eagle_tomahawk.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:free()
        return
    end
    if not battle.paused() then
        me:update_sprite_while_paused()
    end
end

return eagle_tomahawk
