--!strict
-- TomahawkMan's axe (the original's effect object #0x32, `sub_80E3FD0`):
-- the tomahawk he swings for his ETomahwk, riding on him, wearing his
-- header flags and his look. Its action 1 (his routine sets it) swings it
-- (animation 7), 2 ends it; so do the battle's end and his leaving the
-- action that put it on (the original names that action by number,
-- `byte_80E40C0`). Ending, it clears where he keeps it. See
-- docs/engine/standard-chips.md, "Action 0x0A".

local axe = {}

export type State = {
    -- The action he put it on in.
    action: Action?,
}

local SPRITE = asset.sprite("bn6:eagle-tomahawk")
local ANIM_RAISED, ANIM_SWUNG = 6, 7

-- Its actions, which his routine sets.
axe.SWING, axe.END = 1, 2

-- What keeps it: its owner's running action's state, whose `axe` field
-- the game's spawner hands it (the attack variables' word +0x30), and
-- which it clears as it goes.
export type Keeper = { axe: Object? }

-- `*ExtraVars = 0`: clear where its owner keeps it. The game writes the
-- attack variables' word +0x30 whatever its owner runs by then: his
-- chip's `axe` while he is still in it, else the attack's marker word.
local function let_go(me: Object, owner: Object)
    if owner:navi_action() == (me.state :: State).action then
        (owner.attack_state :: Keeper).axe = nil
    else
        owner.marker = 0
    end
end

-- `sub_80E403E`: on its owner, in his flags and look; swung, or gone.
local function tick(me: Object)
    local owner = me.related1 :: Object
    me.pos = owner.pos
    me:copy_visibility(owner)
    me.run_while_paused = owner.run_while_paused
    me.no_sprite_update = owner.no_sprite_update
    me.run_while_dimmed = owner.run_while_dimmed
    me.holds_reservation = owner.holds_reservation
    local look, worn = me.sprite, owner.sprite
    look.color_shader = worn.color_shader
    look.white = worn.white
    look.mosaic = worn.mosaic
    look.alpha = worn.alpha
    if not battle.over() and owner:navi_action() == (me.state :: State).action then
        local a = me.action
        if a == axe.SWING then
            me.anim = ANIM_SWUNG
            me.anim_loaded = 0xFF
            me.action = 0
            return
        elseif a ~= axe.END then
            return
        end
    end
    let_go(me, owner)
    me:set_lifecycle("destroy")
end

-- `sub_80E3FFC`: raised over him.
local function init(me: Object)
    me:set_panel_from_coordinates()
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = ANIM_RAISED
    me.anim_loaded = ANIM_RAISED
    sprite:set_animation(ANIM_RAISED)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

axe.kind = define.kind {
    id = "bn6:tomahawkman/axe",
    pool = "effect",
    state = { action = "action" },
    -- `sub_80E3FD0`: its sprite steps unless the battle is paused.
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
        if not battle.paused() then
            me:update_sprite_while_paused()
        end
    end,
}

-- `sub_80E40C2`: the axe on `owner`, in the action he is running, which
-- keeps it in its state's `axe` (where a failed spawn leaves what was
-- there).
function axe.put_on(owner: Object, keeper: Keeper)
    local o = battle.spawn(axe.kind, owner.pos)
    if o then
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
        local s = o.state :: State
        s.action = owner:navi_action() :: Action
        keeper.axe = o
    end
end

return axe
