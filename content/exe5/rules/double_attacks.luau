--!strict
-- The DS's Double Attacks in the party battle (docs/design/exe5ds-map.md §5): a switch out of Full Synchro to the
-- partner its navi pairs with fires their Double Attack (the chips of ../double_attacks, each with its two navis:
-- lib/double_attack), once a round (the side's state: the DS's AIData +0x73, which nothing clears in a round).
--
-- - **In battle** (the switch's settle, 0x0202C8F8): the launcher (0x02042DC0) at once.
-- - **While paused** (the turn's start, 0x0202C130): the trigger (0x02042E74) keeps the partner and asks for the
--   follow-up (the DS's AIData +0x44 0x02000000), whose action (0x5C, 0x02042ED8: `follow_up`) dims, lands the navi,
--   waits and calls the launcher once the fight goes on.
--
-- The launcher: the outgoing navi in Full Synchro (its mood 0xFF as it left), the side's Double Attack not spent, no
-- dark MegaMan in the party (his block's mood 0), and a Double Attack of the pair; then the chip's director with its
-- damage at the side's level and the chip's hit parameter, a dimming the other side can't cut in on (`sub_800BF16`
-- with 1), its telop the chip's, and the Double Attack spent. The trigger asks the same but for the spent check's
-- place, and adds the DS's light test, which compares MegaMan's light/dark value with his member number (0x02042EAC:
-- it loads 470 and compares the slot): kept as the DS runs it (the user's decision of 2026-10-20), it passes for any
-- value.
--
-- (Battle mode 6's call of the partner without a swap, 0x0203C642, is the offline story's: nettai's battles have no
-- such mode.)

local chips = require("../double_attacks")
local effects = require("../lib/effects")
local element = require("@exelib/element")
local exe5 = require("./api")
local lib = require("../lib/double_attack")
local regions = require("@exelib/regions")

local double_attacks = {}

-- The side's (rules/init.luau's `double_attack`): AIData +0x73, the Double Attack spent this round (0xFF), else on the
-- paused path the outgoing member the follow-up launches with.
type State = { double_attack: { spent: boolean, partner: number } }

local FULL_SYNCHRO = 0xFF

local function state(side: number): { spent: boolean, partner: number }
    local s = rules.state_of(side) :: State?
    if s == nil then
        error(string.format("side %d plays by no rules: no Double Attack state", side))
    end
    return s.double_attack
end

-- The pair table (0x020E0A94, by the navis' numbers, symmetric): the chip whose Double Attack `a` and `b` make.
local function of_pair(a: NaviDef, b: NaviDef): Chip?
    for _, chip in chips do
        local navis = ((chip :: any).double_attack :: DoubleAttack).navis
        if (navis[1] == a and navis[2] == b) or (navis[1] == b and navis[2] == a) then
            return chip
        end
    end
    return nil
end

-- A dark MegaMan in the party (his block's mood, NaviStats +0x0E, 0) stops it.
local function dark_megaman(side: number): boolean
    local m = lib.megaman_member(side)
    if m == nil then
        return false
    end
    local block = battle.member(side, m)
    return block ~= nil and block.mood == 0
end

-- 0x02042DC0: the launcher, for `navi`, which came in for `partner`, whose mood as it left was `mood`.
local function launch(side: number, navi: Object, partner: NaviDef, mood: number)
    if mood ~= FULL_SYNCHRO then
        return
    end
    local s = state(side)
    if s.spent then
        return
    end
    if dark_megaman(side) then
        return
    end
    local chip = of_pair(partner, battle.navi(side).navi)
    if chip == nil then
        return
    end
    local attack = (chip :: any).double_attack :: DoubleAttack
    -- sub_8010D18: the side's level; its row at it.
    local level = exe5.navi_level(side)
    if level == nil then
        error(string.format("side %d states no level: a Double Attack's damage row reads at it", side))
    end
    local damage = attack.damage[level + 1]
    if damage == nil then
        error(string.format("a Double Attack's damage row read at level %d", level))
    end
    local director = attack.spawn(navi, element.byte(chip.element :: string), damage + (chip.hit_param :: number) * 0x10000)
    dimming.start(side, true, director, navi, { chip = chip })
    s.spent = true
end

-- 0x02042E74: the trigger, on the paused path.
local function triggers(side: number, partner: NaviDef, mood: number): boolean
    if mood ~= FULL_SYNCHRO then
        return false
    end
    if state(side).spent then
        return false
    end
    local m = lib.megaman_member(side)
    if m ~= nil then
        local block = battle.member(side, m)
        if block ~= nil and block.mood == 0 then
            return false
        end
        -- (The DS's light test: the value against the member number.)
        if exe5.light_dark_value(side) < m then
            return false
        end
    end
    return of_pair(battle.navi(side).navi, partner) ~= nil
end

-- The rules' `navi_switched`: a party's switch settled, `navi` in for `member`, whose mood as it left was `mood`.
function double_attacks.navi_switched(side: number, navi: Object, member: number, mood: number)
    local out = battle.member(side, member)
    if out == nil then
        error(string.format("side %d switched out of member %d, which its party hasn't", side, member))
    end
    if not battle.paused() then
        launch(side, navi, out.navi, mood)
        return
    end
    if triggers(side, out.navi, mood) then
        state(side).partner = member
        navi:set_request("follow_up", true)
    end
end

-- Action 0x5C's steps: the land, then the wait and the launch.
local LAND, LAUNCH = 0x0, 0x4
local LAND_TICKS, WAIT_TICKS = 0x14, 0x14
local SOUND_SWITCH = asset.sound("cross-change")

type FollowUp = { timer: number }

-- 0x02042EE8: shown white each tick; on its first, the battle dims (battle flag 4 alone), the navi lands on its
-- destination on the ground facing the default way, a flash (the effect table's row 3) 16 pixels up, the switch's
-- sound, no guard, move, flinch, paralysis or slide (nor its request), animation 0, its links and its overlay cut, its
-- collision region the anchor, its anger ended; 20 ticks. (It also zeroes the navi's +0x1F, its slide's state, which
-- the paused switch's land zeroed and nothing set since: the fight hasn't run.)
local function land(me: Object, s: FollowUp)
    me.sprite.white = true
    if me.step_init == 0 then
        battle.set_dimmed(true)
        me:snap_to_future_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        me:face_default()
        battle.effect(Vec3.new(p.x, p.y, 0x100000), effects.deletion)
        battle.play_sound(SOUND_SWITCH)
        for _, flag in { "guard", "moving", "flinching", "paralyzed", "sliding" } :: { StatusFlag } do
            me:set_status(flag, false)
        end
        me:set_request("slide", false)
        me:set_animation(0)
        me.related1 = nil
        me.overlay = nil
        me.collision:set_region(regions.single)
        me:end_anger()
        s.timer = LAND_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = t
    if t <= 0 then
        me.step, me.step_init = LAUNCH, 0
    end
end

-- 0x02042F7C: 21 ticks; then the follow-up's and the trap's states end, the traps' triggers clear, the dimming flag
-- goes, the launcher (the partner the trigger kept, Full Synchro as it found), and back to idle.
local function wait_and_launch(me: Object, s: FollowUp)
    if me.step_init == 0 then
        s.timer = WAIT_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    me:set_navi_state("trap_armed", false)
    me:set_navi_state("following_up", false)
    me:set_request("anti_damage_triggered", false)
    me:set_request("anti_sword_triggered", false)
    battle.set_dimmed(false)
    local side = me.alliance
    local partner = battle.member(side, state(side).partner)
    if partner == nil then
        error(string.format("side %d's Double Attack follow-up names member %d, which its party hasn't", side, state(side).partner))
    end
    launch(side, me, partner.navi, FULL_SYNCHRO)
    me:exit_attack()
end

-- Action 0x5C (0x02042ED8, its table 0x020E0B40): the rules' `follow_up` (rules/roles.luau).
double_attacks.follow_up = new.action {
    id = "double-attack-follow-up",
    state = { timer = "u16" },
    update = function(me: Object, s: FollowUp)
        local step = me.step
        if step == LAND then
            land(me, s)
        elseif step == LAUNCH then
            wait_and_launch(me, s)
        else
            error(string.format("a Double Attack's follow-up: step %#x reads past its table (0x020E0B40)", step))
        end
    end,
}

return double_attacks
