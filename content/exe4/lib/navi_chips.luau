--!strict
-- EXE4's navi chips: dimming chips (action 0x0C) whose variant's spawner (0x080220E0's) makes its navi's controller
-- (each navi its own effect object: TopMan's #0x0B, 0x080E2B54, and the others' copies; SerchMan's, ThunMan's,
-- ProtoMan's, DeltaRay's, MetalMan's and JunkMan's one, #0x42, 0x080E6A18, its navi by the chip's third parameter,
-- here each navi's own): @exelib/navi_chips/controller's, with EXE4's courses (`navi_chips.LONG` and the others) and its warp. The user warps
-- out and back in as EXE4's own effect object #0x0C (0x080E2C24: EXE6's and EXE5's code that neither game runs,
-- `sub_80E11FC`), not EXE6's and EXE5's actor #0x2D (the engine's `navi_chip.warp`).
--
-- A navi chip's palette is its navi's (0x0800B950, `sub_800FCFC`): the navi's palettes per variant times the
-- chip's level's (0x0800B9AC: 0 for a chip of level 0, 2 for an SP's 3, 3 for a DS's 4).

local common = require("@exelib/navi_chips/controller")

local navi_chips = {}

type Step = NaviChipStep

-- The courses (by phase): the user warps out (31 ticks), the navi, then a wait (31) and the user warping back in (31):
-- NumbrMan's (0x080E2F08), FireMan's, GutsMan's, AquaMan's.
navi_chips.LONG = { { wait = 0x1E, warp = "out" }, "navi", { wait = 0x1E }, { wait = 0x1E, warp = "in" } } :: { Step }
-- The same without the wait: effect #0x42's (0x080E6A60: SerchMan's, ThunMan's, ProtoMan's, DeltaRay's, MetalMan's
-- and JunkMan's).
navi_chips.SHARED = { { wait = 0x1E, warp = "out" }, "navi", { wait = 0x1E, warp = "in" } } :: { Step }
-- No warp back: Roll's (0x080E54A0), who brings her user back herself.
navi_chips.RETURNED = { { wait = 0x1E, warp = "out" }, "navi" } :: { Step }
-- The short course (TopMan's 0x080E2B9C and its copies): the user warps out, 11 ticks, the navi, 21 ticks after it
-- left, the undim with the user warping back in.
navi_chips.SHORT = { out = 10, after = 20 } :: NaviChipShortCourse

-- 0x0800B9AC: the palettes a chip's level adds, in its navi's palettes per variant.
local LEVEL_PALETTES: { [number]: number } = { [0] = 0, 0, 0, 2, 3 }

-- 0x0800B950 (`sub_800FCFC`): a navi's palette for its chip's level, the navi having `per_variant` palettes a
-- variant (0x0800B9B1, by the navi).
function navi_chips.palette(per_variant: number, level: number): number
    local n = LEVEL_PALETTES[level]
    if n == nil then
        error(string.format("a navi chip's level %d reads past 0x0800B9AC", level))
    end
    return per_variant * n
end

type WarpState = {
    -- Out (1) or back in (0): Param4.
    out: boolean,
}

-- 0x080E2C44: the navi's image (its sprite by its navi and soul, 0x0800B90A; its palette, `sub_801002C`), where it
-- stands, facing its way, in its animation 4 (out) or 3 (in), shown; then its first tick at once. Only a player's: for
-- another actor (the original's +0x0F neither 0 nor 0xF0 to 0xFC) it just ticks.
local function warp_tick(me: Object)
    local s = me.state :: WarpState
    local navi = me.related1 :: Object
    local player = navi.actor_type == "player"
    if me.phase_init == 0 then
        if s.out then
            if player then
                dimming.hide_user(navi)
            else
                dimming.hide_actor(navi)
            end
        end
        me.phase_init = 4
    end
    if bit32.btest(me.sprite.frame_flags, 0x80) then
        me.visible = false
        if not s.out then
            if player then
                dimming.show_user(navi)
            else
                dimming.show_actor(navi)
            end
        end
        me:set_lifecycle("destroy")
    end
    me:update_sprite_while_dimmed()
end

local function warp_init(me: Object)
    local navi = me.related1 :: Object
    if navi.actor_type == "player" then
        local sprite = me.sprite
        sprite:load_like(navi)
        sprite.shadow = "ground"
        me.visible = true
        me.pos = navi.pos
        sprite.palette = navi.sprite.palette
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me.anim = if (me.state :: WarpState).out then 4 else 3
        me.anim_loaded = 0xFF
    end
    me:set_lifecycle("update")
    warp_tick(me)
end

-- EXE4's warp (effect object #0x0C, 0x080E2C24). (Its init loads its sprite through 0x0800B90A's own table; the
-- engine's `load_like` is the same lookup.)
navi_chips.warp_kind = new.kind {
    id = "navi-chip-warp",
    pool = "effect",
    state = { out = "bool" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            warp_init(me)
        elseif l == "update" then
            warp_tick(me)
        else
            me:free()
        end
    end,
}

-- 0x080E2D26: side `side`'s navi (BattleState +0x44 by the side) warps out (`out`) or back in. (Its position is the
-- spawner's registers, which its init overwrites for a player.)
function navi_chips.warp(side: number, out: boolean)
    local navi = battle.player(side)
    local o = battle.spawn(navi_chips.warp_kind, Vec3.zero)
    if not o then
        return
    end
    if not navi then
        error("a navi chip's user warps, but its side has no navi (0x080E2D26 reads through a null pointer)")
    end
    o.alliance = side
    o.related1 = navi
    ;(o.state :: WarpState).out = out
end

-- A DS navi chip's damage by the field's holes (formula 23 on, 0x08019518): its row's entry 10 less the holes (ten at
-- most), as a count formula's `by_count` (the row reversed).
function navi_chips.by_holes(row: { number }): { number }
    local out = {}
    for i = #row, 1, -1 do
        table.insert(out, row[i])
    end
    return out
end

-- The Roll chips' spawner (0x080E5516): unless the battle is dimmed, the other side's armed AntiRecv (its defensive-
-- chip record's chip, 0x93) springs instead of `hook`: the trap's mark over the user, the record spent, and
-- AntiRecv's counterattack against the user, three times half the chip's damage word (0x080E556C), which starts the
-- side's dimming with no cut-in (`sub_800BF16`).
function navi_chips.anti_recovery(
    hook: (user: Object, spec: DimmingChipSpec) -> Object?
): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        if not battle.dimmed() then
            local trap = battle.linked(bit32.bxor(user.alliance, 1)).chip
            if trap and trap.trap == "anti_recovery" then
                local damage = int.u32(bit32.rshift(spec.damage, 1) * 3)
                local c = navi_chip.spring_anti_recovery(user, damage)
                dimming.start(user.alliance, true, c, user, nil)
                -- (The dimming handler then finds the side's dimming registered and leaves it, its telop
                -- AntiRecv's: the hook hands it no controller to register.)
                return nil
            end
        end
        return hook(user, spec)
    end
end

export type Spec = {
    -- The controller kind's id and the navi's name (for the errors past its tables).
    id: string,
    name: string,
    -- The navi for the controller's user, with the damage word (bonus included); nil if it couldn't come.
    spawn: (controller: Object, user: Object, damage: number) -> Object?,
    -- Its course: steps by phase (`navi_chips.LONG`, `SHARED`, `RETURNED`) or the short one (`SHORT`).
    steps: { Step }?,
    short: NaviChipShortCourse?,
}

-- A navi's controller kind and the `dimming` hook of its chips.
function navi_chips.controller(spec: Spec): NaviChipController
    return common.make {
        id = spec.id,
        name = spec.name,
        spawn = spec.spawn,
        steps = spec.steps,
        short = spec.short,
        warp = navi_chips.warp,
    }
end

return navi_chips
