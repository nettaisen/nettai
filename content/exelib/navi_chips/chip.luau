--!strict
-- A navi chip of EXE6's and EXE5's (EXE6's action 0x1B, EXE5's 0x41, `sub_80EC350`: the user goes back to idle at
-- once): its own controller (the original's effect object #0x10, `sub_80E17E8`, one for every navi chip: compat maps
-- each chip's to it), @exelib/navi_chips/controller's with the two games' course (`sub_80E1830`), and its navi, the
-- chip's own summon (`off_802CD5C[subtype]` with the chip's parameters). A chip's `navi` is what `make` hands back:
-- the controller's spawner (`hook`), its navi's summon and its course, which EXE5's DethPhnx reads to bring the last
-- navi chip's navi again, and what else the chip's spec says (for bn6battle-verify's gen-content to check against the
-- ROM's records).
--
-- The course: the user warps out (31 ticks), the navi until it leaves (`navi_chip.navi_left`), 31 ticks, the user
-- warps back in (31 ticks), then the undim (`PLAIN`); a chip whose user stays on the field warps it neither way
-- (BigHook, the original's navi 0x17: `USER_STAYS`), one whose navi brings the user back itself doesn't warp it back
-- in (Roll's, navi 0: `USER_RETURNED`); either ends a tick after the last wait (`sub_80E18F8`'s first). A chip whose
-- navi heals (Roll's) springs the other side's armed AntiRecv instead of its controller coming (`sub_80E192C`).

local controller = require("./controller")

local chip = {}

-- A chip's navi: for `user`, reporting to `controller`, on the panel, with the element and the damage word (bonus
-- included) of `spec` (`off_802CD5C[subtype]`).
export type Summon = NaviChipSummon

-- `sub_80E1854`, `sub_80E1880`, `sub_80E18DA` and `sub_80E18F8`.
chip.PLAIN =
    { { wait = 0x1E, warp = "out" }, "navi", { wait = 0x1E }, { wait = 0x1E, warp = "in" } } :: { NaviChipStep }
chip.USER_STAYS = { { wait = 0x1E }, "navi", { wait = 0x1E }, { wait = 0 } } :: { NaviChipStep }
chip.USER_RETURNED = { { wait = 0x1E, warp = "out" }, "navi", { wait = 0x1E }, { wait = 0 } } :: { NaviChipStep }

export type Spec = {
    -- Its controller kind's id.
    id: string,
    summon: Summon,
    -- Its course (nil: `PLAIN`).
    course: { NaviChipStep }?,
    -- Whether it is in the navi chips' block (AntiNavi turns it back; EXE5 keeps it as the last navi chip used): nil,
    -- by its `navi` flag (the original's chips 0xDD to 0x118: EXE5's version navi chips are past them, the US games'
    -- records of Django's chips lack the flag).
    navi_block: boolean?,
    -- Its navi heals: the other side's armed AntiRecv springs instead.
    heals: boolean?,
}

-- What a game gives its navi chips: what it keeps of a chip of the block as its navi comes (EXE5's last navi chip).
export type Game = { keep: ((chip: Chip?, element: number, damage: number) -> ())? }

-- `sub_80E199A`: AntiRecv's counterattack's damage word: three times the chip's damage (its low 11 bits, doubled first
-- with the double-damage flag, 0x8000), with the trap's hit parameter (0x1E).
local function counterattack(word: number): number
    local d = bit32.band(word, 0x7FF)
    if bit32.btest(word, 0x8000) then
        d *= 2
    end
    return d * 3 + 0x1E * 0x10000
end

-- A game's navi chips: `make(spec)` is a chip's `navi` (`NaviChip`, types.d.luau).
function chip.maker(game: Game): (spec: Spec) -> NaviChip
    return function(spec: Spec): NaviChip
        local summon = spec.summon
        local course = spec.course or chip.PLAIN
        local c = controller.make {
            id = spec.id,
            name = spec.id,
            -- sub_80E1880: the navi on the controller's panel.
            spawn = function(me: Object, user: Object, damage: number): Object?
                local at = { panel_x = me.panel_x, panel_y = me.panel_y, element = me.element, damage = damage }
                return summon(user, me, at)
            end,
            steps = course,
            -- sub_80E1332
            warp = function(me: Object, out: boolean)
                navi_chip.warp(me.related1 :: Object, out)
            end,
            navi_block = spec.navi_block,
            keep = game.keep,
        }
        local hook = c.hook
        if spec.heals then
            local brings = hook
            -- sub_80E192C: against the other side's armed AntiRecv, the trap springs (`loc_80E1968`, AntiRecv's
            -- counterattack with three times the damage, its telop AntiRecv's: `sub_80E37D2`'s r7, 0xBD), which the
            -- chip's action registers as the side's dimming.
            hook = function(user: Object, dspec: DimmingChipSpec): Object?
                local trap = battle.linked(bit32.bxor(user.alliance, 1)).chip
                if trap and trap.trap == "anti_recovery" then
                    return navi_chip.spring_anti_recovery(user, counterattack(dspec.damage), true)
                end
                return brings(user, dspec)
            end
        end
        return {
            kind = c.kind,
            hook = hook,
            summon = summon,
            course = course,
            navi_block = spec.navi_block,
            heals = spec.heals,
        }
    end
end

return chip
