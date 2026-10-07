--!strict
-- Sparkles (effect object #0x11: EXE6's `sub_80E19BC`, EXE5's 0x080E1A68, EXE4's 0x080E3328, the same code but for
-- where they show): unseen itself, it sets off an effect over an object, jittered, every 16 ticks for a while (VDoll's
-- curse marks the combatants it hits with them; EXE4's SeekBom bursts with them). See docs/engine/shot-chips.md §14.2.
--
-- (EXE6's and EXE5's spawner can also hand it a slot that holds it, and it goes when the slot is cleared; the curse
-- hands it none, and the other spawners, which do, aren't ported. EXE4's has no slot.) A maker taking a game's kind
-- id and sound, and whether its sparkles stay where the object was as they started (EXE4's) or follow it.

local sparkles = {}

-- A game's sparkles: its kind's id, the sound each sparkle makes (EXE6's and EXE5's 0x6F, EXE4's 0x70), and whether
-- they show where the object was as they started (EXE4's init, 0x080E3348, keeps its position at +0x60) rather than
-- where it is at each sparkle (none: EXE6's and EXE5's).
export type SparklesLook = { id: string, sound: Sound, stays: boolean? }

-- The sparkles made for a game: `spawn` is `sub_80E1A6A`, sparkles over `over` (their position is what the
-- spawner's registers held; nothing draws them).
export type Sparkles = {
    kind: Kind,
    spawn: (over: Object, ticks: number, effect: Effect, silent: boolean) -> Object?,
}

export type State = {
    -- How long it lasts (Param1, by `dword_80E19F0`), the effect it shows
    -- (Param2), and silent (Param3: no sound with it).
    ticks: number,
    effect: Effect,
    silent: boolean,
    -- Where they show, a staying look's (EXE4's +0x60: its object's position as it started).
    at: Vec3?,
}

-- The effect shows when its timer's low four bits are all set (the routine
-- means a mask of 3 for length 0, but its test reads stale flags), jittered
-- by up to 15 pixels and 16 pixels up.
local MASK, JITTER, LIFT = 0xF, 0x1F, 0x10_0000

function sparkles.make(look: SparklesLook): Sparkles
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
        local at = if look.stays then s.at :: Vec3 else (me.related1 :: Object).pos
        local p = battle.jitter(JITTER, at)
        battle.effect(Vec3.new(p.x, p.y, int.i32(p.z + LIFT)), s.effect)
        if not s.silent then
            battle.play_sound(look.sound)
        end
    end

    local schema: StateSchema = { ticks = "u8", effect = "effect", silent = "bool" }
    if look.stays then
        schema.at = "vec3"
    end
    local kind = new.kind {
        id = look.id,
        pool = "effect",
        state = schema,
        -- `sub_80E19BC`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                -- `sub_80E19DC` (EXE4's 0x080E3348 keeps its object's position).
                local s = me.state :: State
                me.timer = s.ticks
                if look.stays then
                    s.at = (me.related1 :: Object).pos
                end
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

    -- `sub_80E1A6A`.
    local function spawn(over: Object, ticks: number, effect: Effect, silent: boolean): Object?
        local o = battle.spawn(kind, Vec3.zero)
        if o then
            local s = o.state :: State
            s.ticks, s.effect, s.silent = ticks, effect, silent
            o.related1 = over
        end
        return o
    end

    return { kind = kind, spawn = spawn }
end

return sparkles
