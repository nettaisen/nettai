--!strict
-- Sparkles (the original's effect object #0x11, `sub_80E19BC`): unseen
-- itself, it sets off an effect over an object, jittered, every 16 ticks
-- for a while (VDoll's curse marks the combatants it hits with them). See
-- docs/engine/shot-chips.md §14.2.
--
-- (Its spawner can also hand it a slot that holds it, and it goes when the
-- slot is cleared; the curse hands it none, and the other spawners, which
-- do, aren't ported.)

local sparkles = {}

export type State = {
    -- How long it lasts (Param1, by `dword_80E19F0`), the effect it shows
    -- (Param2), and silent (Param3: no sound with it).
    length: number,
    effect: Effect,
    silent: boolean,
}

-- Its lifetimes (`dword_80E19F0`), by its first parameter.
local LENGTHS: { [number]: number } = { [0] = 0x20, [1] = 0x5A, [2] = 0x20, [3] = 0x00 }
-- The effect shows when its timer's low four bits are all set (the routine
-- means a mask of 3 for length 0, but its test reads stale flags), jittered
-- by up to 15 pixels and 16 pixels up.
local MASK, JITTER, LIFT = 0xF, 0x1F, 0x10_0000
local SOUND = asset.sound("hit-bomb-0")

-- `sub_80E1A0C`: count down; on the ticks the mask matches, the effect
-- over the object.
local function sparkle(me: Object)
    local s = me.state :: State
    local left = me.timer - 1
    if left <= 0 then
        me.timer = bit32.band(left, 0xFFFF)
        me:set_lifecycle("destroy")
        return
    end
    me.timer = left
    if bit32.band(left, MASK) ~= MASK then
        return
    end
    local over = me.related1 :: Object
    local p = battle.jitter(JITTER, over.pos)
    battle.effect(Vec3.new(p.x, p.y, int.i32(p.z + LIFT)), s.effect)
    if not s.silent then
        battle.play_sound(SOUND)
    end
end

sparkles.kind = define.kind {
    id = "vdoll/sparkles",
    pool = "effect",
    state = { length = "u8", effect = "effect", silent = "bool" },
    -- `sub_80E19BC`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- `sub_80E19DC`.
            local s = me.state :: State
            local t = LENGTHS[s.length]
            if t == nil then
                error(string.format("sparkles length %d reads past dword_80E19F0", s.length))
            end
            me.timer = t
            me:set_lifecycle("update")
        elseif l == "update" then
            if me.action ~= 0 then
                error(string.format("sparkles action %#x reads past off_80E1A08", me.action))
            end
            sparkle(me)
        else
            -- `sub_80E1A5E`.
            me.phase_init = 1
            me:free()
        end
    end,
}

-- `sub_80E1A6A`: sparkles over `over` (their position is what the
-- spawner's registers held; nothing draws them).
function sparkles.spawn(over: Object, length: number, effect: Effect, silent: boolean): Object?
    local o = battle.spawn(sparkles.kind, Vec3.zero)
    if o then
        local s = o.state :: State
        s.length, s.effect, s.silent = length, effect, silent
        o.related1 = over
    end
    return o
end

return sparkles
