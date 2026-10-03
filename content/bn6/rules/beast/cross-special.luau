--!strict
-- BN6's Cross special: DarkInvs' auto-battle in the battle flag 0x40 mode.
-- DarkInvs asks for a takeover of its user (the request "takeover"); idle
-- hands the request to the side's systems (`sub_802E4E4`), and the beast
-- system starts it: 0x1E0 ticks (`battle.take_over`), the navi
-- invulnerable for good, the controllers' state cleared. While it runs, its
-- controller (`sub_802D4C6`) moves next to an opponent and attacks with
-- chips of its own (the section `bn6:cross-special`'s rows, `sub_802D5A8`),
-- with a buster volley while an opponent stays in its row; when its ticks
-- run out it ends (`sub_80EFDB2`, the action below: compat 0x59). Its state
-- is the berserk's (berserk.luau): the original's controllers share the
-- navi's AIData +0xF0.

local berserk = require("./berserk")
local effects = require("../../lib/effects")
local regions = require("@common/regions")
local section = require("../cross-special")

local cross_special = {}

-- The beast system's state fields the controllers share (rules/beast/
-- system.luau declares them; berserk.luau's).
type State = {
    controller_step: number,
    controller_moves: number,
    controller_volley: number,
    controller_started: boolean,
}

-- A chip's record damage (`ChipData`'s: a formula's reads 0).
local function record_damage(chip: Chip): number
    local d = chip.damage
    return if type(d) == "number" then d else 0
end

-- The special's length.
local TICKS = 0x1E0
-- The sound its first chip makes (`sub_802D4F0`).
local SOUND_START = asset.sound("bn6:elmnt-man-2")

-- `sub_802E4E4`'s start of it.
function cross_special.requested(side: number, navi: Object)
    local s = system.state() :: State
    battle.take_over(side, TICKS)
    navi:set_status_timer("invulnerable", 0xFFFF)
    navi:set_status("invulnerable", true)
    berserk.reset(s)
end

-- `sub_802D5A8`: a chip for the special, at random from its navi's row (by
-- the hundreds of the base max HP, NaviStats+0x3E: the first row up to
-- 199, the tenth from 1000), with the chip whose damage it strikes with,
-- if another's.
local function pick_chip(side: number): (Chip, Chip?)
    local rows = section.rows
    local hundreds = battle.navi(side).max_base_hp // 100
    local row = math.min(math.max(hundreds - 1, 0), #rows - 1)
    local chips = rows[row + 1]
    if chips == nil or #chips == 0 then
        error(string.format("the Cross special has no chips for row %d (rules cross-special)", row))
    end
    local entry = chips[battle.rng_positive() % #chips + 1]
    if entry.chip ~= nil then
        return entry.chip, entry.damage_of
    end
    return entry :: any, nil
end

-- `sub_802D4F0`: (the first time, a sound) for the first three moves
-- after an attack, wait with a volley while the opponent under the target
-- marker flashes; otherwise use one of the special's chips (not the hand's:
-- the attack comes from its record, and the chip isn't spent) and move
-- next.
local function special_chip(side: number, navi: Object, s: State): "nothing" | "own_chip"
    if not s.controller_started then
        s.controller_started = true
        battle.play_sound(SOUND_START)
    end
    if s.controller_moves <= 3 then
        local x, y = berserk.NO_MARKER_X, berserk.NO_MARKER_Y
        local marker = navi.target_marker
        if marker ~= nil then
            x, y = marker.panel_x, marker.panel_y
        end
        local t = berserk.opponent_on(x, y, navi.alliance)
        if t ~= nil and t:status("flashing") then
            s.controller_step = berserk.BUSTER
            -- The game means 6 (2 in Beast Out) but stores the form's
            -- number (a register mixup), which is what a form's
            -- `special_volley` says: in the base form (0) the volley lasts
            -- until the row is clear (or 65536 shots).
            s.controller_volley = battle.navi(side).form.special_volley or 0
            return "nothing"
        end
    end
    s.controller_moves = 0
    local chip, damage_of = pick_chip(side)
    navi.attack_chip = chip
    -- (The original copies the record's subtype and parameter bytes too:
    -- a chip has neither here; what its action needs is its definition's.)
    navi.attack_damage = record_damage(chip)
    navi.hit_param = bit32.bor(chip.hit_param or 0, 0x80)
    -- (The last rows' LifeSrd strikes with VarSwrd's damage.)
    if damage_of ~= nil then
        navi.attack_damage = record_damage(damage_of)
    end
    navi:start_chip_attack(chip, 5)
    local beast = chip.beast
    navi.wrapped = if beast ~= nil and beast.rush ~= false then 1 else 0
    s.controller_step = berserk.MOVE
    return "own_chip"
end

-- `sub_802D588`: the volley: the buster fires while an opponent is in the
-- navi's row and shots are left; then a move is next.
local function volley(navi: Object, s: State): "nothing" | "buster"
    if berserk.opponent_in_row(navi, navi.panel_y) ~= nil then
        s.controller_volley = (s.controller_volley - 1) % 0x10000
        if s.controller_volley ~= 0 then
            return "buster"
        end
    end
    s.controller_step = berserk.MOVE
    return "nothing"
end

-- `sub_80EFDB2`, action 0x59: the special is over (its ticks ran out). The
-- navi flashes white, lands where it was going with its collision off,
-- and after 30 ticks gets its collision back, loses the invulnerability
-- and pending attacks, flashes for 120 ticks, and its HP bug worsens by 4
-- (at most 7).
local TICKS_LANDING = 0x1E
local SOUND_END = asset.sound("bn6:fade")
-- The effect over the navi (BN6's role `effects.deletion`), 20 pixels up.
local LOOK_Z = 0x14 * 0x10000

type EndState = { timer: number }

local function finish(me: Object, s: EndState)
    -- sprite_forceWhitePalette (every tick).
    me.sprite.white = true
    if me.step_init == 0 then
        battle.play_sound(SOUND_END)
        me.collision:set_region(nil)
        me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
        me:reserve_panel(me.panel_x, me.panel_y)
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        me:face_default()
        local p = me.pos
        local e = battle.effect(Vec3.new(p.x, p.y, p.z + LOOK_Z), effects.deletion, 0, 0, 0)
        if e ~= nil then
            e.run_while_paused = true
        end
        for _, flag in { "bubbled", "frozen", "sliding", "paralyzed", "flinching", "moving" } :: { StatusFlag } do
            me:set_status(flag, false)
        end
        me:set_request("slide", false)
        me:set_navi_state("heat_trap", false)
        s.timer = TICKS_LANDING
        me.step_init = 4
    end
    local left = s.timer - 1
    s.timer = left % 0x10000
    if left > 0 then
        return
    end
    -- (BN6's role `regions.anchor`.)
    me.collision:set_region(regions.single)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    for _, r in { "mode9_a", "alt_chip", "back_special", "charged_chip", "chip", "buster", "forced_charged_shot" } :: { Request } do
        me:set_request(r, false)
    end
    -- sub_800EB08
    me:set_status_timer("invulnerable", 0)
    me:set_status("invulnerable", false)
    -- sub_801A66C: 120 ticks of flashing.
    me:set_status_timer("flash", 0x78)
    me:set_status("flashing", true)
    local stats = battle.navi(me.alliance)
    stats.hp_drain = math.min(stats.hp_drain + 4, 7)
    me:exit_attack()
end

cross_special.action = define.action {
    id = "bn6:beast/cross-special-end",
    state = { timer = "u16" },
    update = function(me: Object, s: any)
        finish(me, s :: EndState)
    end,
}

-- `sub_802D4C6`: one tick of the special's controller, from the idle
-- action; when its ticks have run out, its end.
function cross_special.takeover(side: number, navi: Object): ("nothing" | "own_chip" | "buster" | "moved")?
    local s = system.state() :: State
    -- (`sub_802D310` cleared the shared state: it starts over.)
    if navi.controller_fresh then
        navi.controller_fresh = false
        berserk.reset(s)
    end
    if battle.takeover_ticks(side) == 0 then
        battle.end_takeover(side)
        navi:set_attack(cross_special.action, 0)
        return "nothing"
    end
    if s.controller_step == berserk.USE_CHIP then
        return special_chip(side, navi, s)
    elseif s.controller_step == berserk.BUSTER then
        return volley(navi, s)
    end
    return berserk.step_toward_opponent(navi, s)
end

return cross_special
