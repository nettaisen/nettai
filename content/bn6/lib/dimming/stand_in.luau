--!strict
-- The stand-ins some dimming chips send out in their user's place
-- (docs/engine/dimming-chip-effects.md §2, §10, §11, §15, §16): an actor
-- drawn as the user (its sprite, palette and parts) while the user
-- vanishes, which gives the user back when it is done and tells its
-- controller so. BurnSqr's burner, BlzrdBal's thrower, Magnum's gunner,
-- NumbrBl's NumberMan and CornFsta's farmer are stand-ins; each is its
-- chip's own kind, whose state has `controller` (`stand_in.STATE`), and
-- whose controller's state has `busy`.

local stand_in = {}

-- The state fields every stand-in kind has: the controller waiting on it
-- (the original's flag pointer, ExtraVars+0, at the controller's second
-- parameter).
export type State = { controller: Object? }

-- What a controller waiting on a stand-in keeps: it is out (the original's
-- second parameter, which the stand-in's spawn sets and its end clears).
export type Waiting = { busy: boolean }

-- `sub_80BB13E` and its copies (`sub_80B8BA0`, `sub_80BB49C`,
-- `sub_80BCCDC`, `sub_80C31F0`): a stand-in of `kind` for `user` on
-- `controller`'s panel, with its element and the damage word `damage`, of
-- the user's side and facing; `controller` is busy until it ends. Its
-- position is the spawner's registers until its init copies the user's.
-- (The spawners but NumbrBl's copy the user's NameID too, which nothing
-- reads in a stand-in.)
function stand_in.spawn(kind: Kind, controller: Object, user: Object, damage: number): Object?
    local o = battle.spawn(kind)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = controller.panel_x, controller.panel_y
    o.element = controller.element
    o:set_damage_word(damage)
    o.related1 = user
    ;(o.state :: State).controller = controller
    ;(controller.state :: Waiting).busy = true
    o.alliance, o.flip = user.alliance, user.flip
    o.run_while_dimmed = true
    return o
end

-- `sub_80BAF74` and its copies: drawn as the user (its sprite, with a
-- shadow, from animation 0; its palette and parts, which step even while
-- paused), where the user is; the user vanishes, its status visuals kept.
function stand_in.init(me: Object): Object
    local user = me.related1
    if not user then
        error("a dimming chip's stand-in has no user (its init reads through a null pointer)")
    end
    local sprite = me.sprite
    sprite:load_like(user)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0xFF
    me.visible = true
    me.pos = user.pos
    me:set_panel_from_coordinates()
    -- sub_801002C: the user's navi palette.
    sprite.palette = user.sprite.palette
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:add_parts_of(user, false, true)
    dimming.hide_user_sparing(user)
    return user
end

-- Shown, unless the local player is blind to its side (`sub_800EB6C`).
function stand_in.show(me: Object)
    me.visible = battle.viewer_sees(me.alliance)
end

-- Done (`sub_80BB10C` and its copies): its parts off, hidden, freed at its
-- next update; the user back; the controller goes on. (The original clears
-- the flag wherever the controller was; it is still there in every case
-- the chips reach.)
function stand_in.leave(me: Object)
    local user = me.related1 :: Object
    me:remove_parts_of(user)
    me.visible = false
    me:set_lifecycle("destroy")
    dimming.show_user(user)
    local c = (me.state :: State).controller
    if c and c.active then
        (c.state :: Waiting).busy = false
    end
end

return stand_in
