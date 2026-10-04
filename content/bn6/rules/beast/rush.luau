--!strict
-- BN6's Beast Out rush (`sub_80EAD9C`): the game's wrapper (the role
-- `actions.wrapper`), which the navi's dispatcher (`sub_801B9E6`) runs
-- instead of an attack while the attack's `wrapped` is 1: a chip with
-- the lock-on flag in a Beast form, the Beast forms' charged claw and
-- SlashCross Beast's charged sword. The navi holds its panel, warps next to
-- the lock-on marker's target, runs the chip's action there (`run_wrapped`)
-- and warps back to its panel. An A press during the rush queues the next
-- chip, which then warps and attacks in turn without going home first
-- (chips.md §2.11 and §4.6). Its state is the beast system's (the
-- original's attack variables +0x1E to +0x27, next to the chip action's
-- own).

local beast_claw = require("../../navis/megaman/weapons/beast-claw")
local slash_charge = require("../../navis/megaman/forms/slashcross/charge")
local kind = require("../forms/kind")
local lockon = require("../lockon")

local rush = {}

-- The beast system's state fields the rush keeps (rules/beast/init.luau
-- declares them).
type State = {
    -- The phase: hold the panel, warp, attack.
    rush_phase: number,
    -- The phase's entry ran.
    rush_started: boolean,
    -- The panel the warp left.
    rush_home_x: number,
    rush_home_y: number,
    -- An A press queued the next chip.
    rush_chain: boolean,
    -- Ticks left before A presses queue.
    rush_chain_block: number,
    -- Ticks left before the warp.
    rush_countdown: number,
}

-- Hold the panel (`sub_80EADDC`); wait, then warp next to the target
-- (`sub_80EAE28`); run the chip's action, then chain or warp back
-- (`sub_80EAF36`).
local HOLD, WARP, ATTACK = 0, 1, 2

-- Ticks after the rush starts before an A press can queue the next chip.
local CHAIN_BLOCK_TICKS = 12

-- Where the lock-on marker's target stands when the navi has no marker
-- (`sub_80E1676`): off the field.
local NO_MARKER_X, NO_MARKER_Y = 0xA0, 0xE3

-- The framework's states an attack isn't (the original's CurAction below
-- 0x10).
local STATES = {
    entry = true,
    take_control = true,
    deletion = true,
    flinch = true,
    paralysis = true,
    drag = true,
    freeze = true,
    bubble = true,
    idle = true,
}

local function set_phase(s: State, phase: number)
    s.rush_phase = phase
    s.rush_started = false
end

-- `sub_80EADDC`: settle on the destination panel and reserve it; the navi
-- counts as moving.
local function hold(navi: Object, s: State)
    navi:set_status("using_action", false)
    navi:snap_to_future_panel()
    navi:set_status("sliding", false)
    navi:set_status("moving", false)
    navi:set_request("slide", false)
    local x, y = navi.panel_x, navi.panel_y
    navi.future_panel_x, navi.future_panel_y = x, y
    navi:reserve_panel(x, y)
    navi:set_status("moving", true)
    s.rush_chain = false
    s.rush_chain_block = CHAIN_BLOCK_TICKS
    set_phase(s, WARP)
end

-- The lock-on mode: none (stay) while blind or confused outside Beast
-- Over; else the claw's own (`sub_80EAF1A`: the role `lockon.beast_claw`),
-- the charged sword's (`sub_80EAF26`: a table by the attack's variant in
-- the original; here the mode the charged sword's setup gave with its
-- slash, `rush_lockon`), and failing those, the chip's.
local function lockon_mode(navi: Object): Lockon?
    local over = kind.beast_over(navi.alliance)
    if not over and (navi:status("blind") or navi:status("confused")) then
        return nil
    end
    local action = navi:navi_action()
    local special: Lockon? = nil
    if action == beast_claw.action then
        special = lockon.beast_claw
    elseif action == slash_charge.action then
        special = navi.rush_lockon
    end
    if special ~= nil then
        return special
    end
    -- (The empty hand's chip, the zeroed one, has no lock-on mode.)
    local chip = navi.attack_chip
    -- (The beast system's extension of chips, BN6's: types.d.luau.)
    local beast = if chip ~= nil then ((chip :: any) :: Bn6ChipFields).beast else nil
    return if beast ~= nil then beast.lockon else nil
end

-- The afterimage's animation: 0xF, or 0 for the link navis.
local function afterimage_anim(navi: Object): number
    local identity = navi.identity
    return if identity ~= nil and identity.class == "link_navi" then 0 else 0xF
end

-- `sub_80E33FA` with Param1 0xFF: a copy of the navi at `pos` for
-- `lifetime` ticks, less green, with a ground shadow, tied to the Beast
-- form (or to the attack outside one: `sub_80E341E`).
local function afterimage(navi: Object, pos: Vec3, anim: number, lifetime: number)
    local beast = kind.beast(navi.alliance)
    battle.afterimage(navi, pos, {
        anim = anim,
        flip = bit32.bxor(navi.alliance, navi.flip),
        lifetime = lifetime,
        color_shader = 0x83E0,
        shadow = "ground",
        tether = if beast then "form" else "attack",
    })
end

-- `sub_80EAE28`: lock on (freeze the marker), wait 3 ticks, then warp to
-- the panel the chip's lock-on mode picks near the target, leaving two
-- afterimages when the navi moved.
local function warp(navi: Object, s: State)
    if not s.rush_started then
        s.rush_started = true
        -- `sub_80E1654` (a no-op without a marker).
        local marker = navi.target_marker
        if marker ~= nil then
            marker:freeze_target_marker(true)
        end
        navi:set_animation(4)
        s.rush_countdown = 3
    end
    s.rush_countdown = (s.rush_countdown - 1) % 0x10000
    if s.rush_countdown ~= 0 then
        return
    end
    local hx, hy = navi.panel_x, navi.panel_y
    navi:panel_trail(hx, hy)
    local mode = lockon_mode(navi)
    local tx, ty = NO_MARKER_X, NO_MARKER_Y
    local marker = navi.target_marker
    if marker ~= nil then
        tx, ty = marker.panel_x, marker.panel_y
    end
    -- (No panel fits: the navi stays.)
    local dx, dy = navi:lockon_panel(tx, ty, mode)
    if dy == 0x7F then
        dx, dy = hx, hy
    end
    s.rush_home_x, s.rush_home_y = hx, hy
    navi.panel_x, navi.panel_y = dx, dy
    navi:set_coordinates_from_panel()
    navi:update_collision_panels()
    local target = navi.face_target
    local pattern = battle.panel_pattern()
    if target ~= nil and (pattern == 0x31 or pattern == 0x23 or pattern == 0x33) then
        navi:face_toward(target)
    end
    if dx ~= hx or dy ~= hy then
        local pos = navi.pos
        local cx, cy = field.center(hx, hy)
        local anim = afterimage_anim(navi)
        afterimage(navi, Vec3.new(cx, cy, pos.z), anim, 12)
        afterimage(navi, Vec3.new((cx + pos.x) // 2, (cy + pos.y) // 2, pos.z), anim, 20)
    end
    navi:set_animation(0)
    set_phase(s, ATTACK)
end

-- `sub_80EAF36`: run the chip's action; when it is back to idle, chain the
-- queued chip or warp home.
local function attack(navi: Object, s: State)
    local action = navi:navi_action()
    if type(action) ~= "string" or not STATES[action] then
        navi:run_wrapped()
        if navi:navi_action() ~= "idle" then
            return
        end
        if s.rush_chain and navi:chain_next_chip() then
            -- `sub_800FC7C`.
            battle.advance_hand(navi.alliance)
            -- (The chained chip's start cleared the attack's links: the rush
            -- goes on.)
            navi.wrapper_fresh = false
            set_phase(s, WARP)
            s.rush_chain = false
            -- (It also writes 0xC to AIData+0x85, which no player code
            -- reads.)
            local marker = navi.target_marker
            if marker ~= nil then
                marker:freeze_target_marker(false)
            end
            return
        end
    end
    navi:reset_attack_links()
    navi:snap_to_future_panel()
    navi:set_status("moving", false)
    navi:set_status("move_complete", true)
    navi:exit_attack()
end

-- `sub_80EAD9C`: one tick of the rush, then the chain window: after it
-- opens, an A press queues the next chip.
local function update(navi: Object, _: any)
    local s = system.state() :: State
    -- (`sub_801011A` cleared the rush with the attack's links: it starts
    -- over.)
    if navi.wrapper_fresh then
        navi.wrapper_fresh = false
        set_phase(s, HOLD)
    end
    if s.rush_phase == HOLD then
        hold(navi, s)
    elseif s.rush_phase == WARP then
        warp(navi, s)
    else
        attack(navi, s)
    end
    if s.rush_chain_block ~= 0 then
        s.rush_chain_block -= 1
    elseif navi:pressed("a") then
        s.rush_chain = true
    end
end

rush.action = define.action {
    id = "beast/rush",
    update = update,
}

return rush
