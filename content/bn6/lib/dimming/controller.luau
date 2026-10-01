--!strict
-- What every dimming chip's controller shares (docs/design/content-model-v2.md
-- §5.4, docs/engine/dimming-chips.md §1): the spawn each `off_802CCB4`
-- entry does, and the update that runs the dimming service's steps in the
-- original's order around the chip's own effect.
--
-- A controller is an effect kind with states 0 (`object_timefreezeBegin`),
-- 4 (its actions) and 8 (`object_timefreezeEnd`), and actions 0
-- (`object_dimScreen`), 4 (`object_drawChipName`: the telop and the wait
-- for a counter cut-in), 8 (the chip's effect) and 0xC
-- (`object_undimScreen`).

local controller = {}

-- What the spawner copies from the user, past its side, element, damage
-- word and itself (the first related object).
export type SpawnOptions = {
    -- The user's panel (all but the barriers' spawner copy it).
    panel: boolean?,
    -- The user's facing (the barriers' spawner copies only its side).
    flip: boolean?,
}

-- `off_802CCB4[n]`'s spawn: a controller of `kind` for `user`, with the
-- chip's element and damage word. It is spawned at the spawner's registers
-- (the user's panel Y, the element, the caller's r3), which nothing reads:
-- compat's `scratch_position` for every controller.
function controller.spawn(kind: Kind, user: Object, spec: DimmingChipSpec, opts: SpawnOptions?): Object?
    local o = battle.spawn(kind, Vec3.zero)
    if not o then
        return nil
    end
    local panel = not (opts and opts.panel == false)
    local flip = not (opts and opts.flip == false)
    if panel then
        o.panel_x, o.panel_y = user.panel_x, user.panel_y
    end
    o.element = spec.element
    o.related1 = user
    o.alliance = user.alliance
    if flip then
        o.flip = user.flip
    end
    o:set_damage_word(spec.damage)
    return o
end

-- The controller's update: its states, and by action the dimming's steps
-- with `effect` at action 8. `name` names the controller in errors.
function controller.update(name: string, effect: (me: Object) -> ()): (me: Object) -> ()
    return function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            dimming.begin(me)
        elseif l == "update" then
            local a = me.action
            if a == 0 then
                dimming.dim_screen(me)
            elseif a == 4 then
                dimming.show_telop(me)
            elseif a == 8 then
                effect(me)
            elseif a == 0xC then
                dimming.undim_screen(me)
            else
                error(string.format("%s's action %#x reads past its table", name, a))
            end
        else
            dimming.finish(me)
        end
    end
end

-- The controller's user (its first related object); `routine` names what
-- would write through the null pointer.
function controller.user(me: Object, routine: string): Object
    local user = me.related1
    if not user then
        error(string.format("%s: the dimming controller has no user (%s reads through a null pointer)", me.kind.id, routine))
    end
    return user
end

-- Count the timer down (a halfword, `ldrh; sub #1; strh`); true once it
-- went below 0 (`bge`: N + 1 updates from N).
function controller.count_to_minus_one(me: Object): boolean
    local t = me.timer - 1
    me.timer = t
    return t < 0
end

-- The same with `bgt`: true once it reached 0 (N updates from N).
function controller.count_to_zero(me: Object): boolean
    local t = me.timer - 1
    me.timer = t
    return t <= 0
end

-- On to the undim (action 0xC, from its first phase).
function controller.undim(me: Object)
    me:set_action(0xC)
end

return controller
