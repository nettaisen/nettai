--!strict
-- DarkInvs's dimming controller (BN5's effect object #0x18, 0x080E22A8; BN5's own): the usual dimming phases
-- (@exelib/dimming), with an effect (0x080E22EC) that sends out a stand-in of its user (./stand-in: the user turning
-- dark), waits for it, and starts the user's drive: for 600 ticks the computer-navi AI drives the user
-- (rules/computer-navi: where it would step, its idle asks the AI, on the user's own side's tactics), which no hit
-- reaches (untouchable), its statuses and anger ended. The drive's end is BN5's action 0x49 (./drive-end), which its
-- timer's end asks for (rules/status's `no_charge_drive`).

local dimming_chips = require("@exelib/dimming")
local standin = require("./stand-in")

export type State = {
    -- The stand-in is out (+0x0C, the flag it was given).
    busy: boolean,
}

local controller = {}

-- The drive's ticks (0x080E231A).
local DRIVE = 0x258

-- 0x0802C03A and 0x0802C110 on the user: the drive's state from the start (the computer-navi system's; the idle's
-- reset, 0x0802C03A each idle without the drive, has left it so: nothing writes it before the drive's first step,
-- where the system starts it over on finding the drive new), the timer, the AI on (AIData +0xF0), the no-charge
-- state (0x200); untouchable (flag 0x08000000: no hit reaches it); its statuses and its anger ended (0x08017068,
-- 0x08011A94).
local function start_drive(user: Object)
    user.no_charge_timer = DRIVE
    user.computer_driven = true
    user:set_navi_state("no_charge", true)
    user:set_status("untouchable", true)
    user:end_statuses()
    user:end_anger()
end

-- 0x080E22EC: on its first tick the stand-in on the user's panel; once it is done, the drive, and the undim.
local function effect(me: Object)
    local s = me.state :: State
    local user = me.related1 :: Object
    if me.phase_init == 0 then
        standin.spawn(me, user, user.panel_x, user.panel_y)
        me.phase_init = 4
    end
    if s.busy then
        return
    end
    start_drive(user)
    dimming_chips.done(me)
end

controller.kind = define.kind {
    id = "bn5:darkinvs/controller",
    pool = "effect",
    state = { busy = "bool" },
    -- 0x080E22A8.
    update = dimming_chips.phases { name = "DarkInvs", effect = effect },
}

-- 0x080E2338: DarkInvs's `dimming` hook (only the alliance byte of its user's).
function controller.hook(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(controller.kind, user, spec, true)
end

return controller
