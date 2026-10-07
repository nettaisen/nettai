--!strict
-- EXE4's spawners, the chips' action 0x20 (0x080EC972): on its first tick the variant's spawner (0x080EC9CC by the
-- variant: Lance's lances, Boomer's boomerang, the Counters, the WhitWebs, Atk+10, Repair and the rest) puts out its
-- objects with the attack's element and damage word (its +6 byte added) at the user's height, and the attack ends;
-- SidBmbo's (variant 5) holds the user 40 ticks first. The user shows no animation of its own. Each chip composes its
-- own with its variant's spawner (its parameters, 0x10's word, the chip's own data).

local spawners = {}

export type Spawner = {
    id: string,
    -- The variant's spawner: the user, the attack's element and damage word.
    spawn: (user: Object, element: number, damage: number) -> (),
    -- How long the user stays in the action after the spawn (SidBmbo's 40); none, the attack ends at once.
    hold: number?,
}

type State = { timer: number }

local SPAWN = 0x0

function spawners.action(spec: Spawner): Action
    return new.action {
        id = spec.id,
        state = { timer = "i16" },
        update = function(me: Object, s: State)
            if me.step == SPAWN then
                me.step = 4
                local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + bit32.band(me.extra, 0xFF))
                spec.spawn(me, me.attack_element, damage)
                if not spec.hold then
                    me:exit_attack()
                    return
                end
                s.timer = spec.hold
            end
            s.timer -= 1
            if s.timer <= 0 then
                me:exit_attack()
            end
        end,
    }
end

return spawners
