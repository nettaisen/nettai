--!strict
-- MegaMan's taunt in EXE4 (`megaman/taunt`): the charged weapon DrkCanon's cost leaves him (rules/dark_chips: NaviStats
-- +0x0A = 0x21, which the weapons take at their next reload, 0x0800BD48), the original's weapon routine 0x21
-- (0x0800CFEC: every attack variable 0) and the action it starts (0x2A, 0x080ED0FE): no shot; on its second tick a
-- mark over him (attachment row 21) with its sound (0xC1), and in a netbattle the other side's navi is angered (its
-- anger request, flag2 0x200, which its next anger tick takes, 0x0800C594); after 5 ticks the recovery, 11 ticks,
-- which a move may cut short, as the charged shot's.

local attachment = require("../../../objects/attachment")

local taunt = {}

local TAUNT, RECOVER = 0x0, 0x4
-- The mark (attachment row 21, 0x080CD524: 0c-1f in palette 0, lifted a pixel).
local MARK = attachment.look { sprite = asset.sprite("taunt"), palette = 0, lift = 1 }
local SOUND = asset.sound("shield")

type State = { timer: number }

-- 0x080ED11C.
local function taunting(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x12)
        me:set_status("using_action", true)
        me.step_init = 4
        s.timer = 0
    end
    if s.timer == 1 then
        battle.play_sound(SOUND)
        attachment.attach(me, MARK, "related", 0)
        if battle.link() then
            local other = battle.player(1 - me.alliance)
            if other then
                other:set_request("anger", true)
            end
        end
    end
    s.timer += 1
    if s.timer > 4 then
        me.step = RECOVER
        me.step_init = 0
    end
end

-- 0x080ED18C: the recovery; its end exits the attack and a move may still start, as may one before.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = 10
        me.step_init = 4
    end
    s.timer -= 1
    if s.timer < 0 then
        me.related1 = nil
        me:exit_attack()
    end
    if not me:can_move() then
        return
    end
    local dir = me:held_direction()
    if dir == 0 or me:step_target(dir) == nil then
        return
    end
    me.related1 = nil
    me:exit_attack()
    me:start_move(dir)
end

taunt.action = new.action {
    id = "megaman/taunt/action",
    state = { timer = "i16" },
    update = function(me: Object, s: State)
        local step = me.step
        if step == TAUNT then
            taunting(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("taunt phase %#x reads past its phase table", step))
        end
    end,
}

-- 0x0800CFEC.
function taunt.setup(navi: Object): Action
    navi.attack_damage = 0
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    return taunt.action
end

taunt.weapon = new.weapon {
    id = "megaman/taunt",
    -- The charge table's row 0x21 (0x0801820A), by Charge 0 to 4; Charge 5 reads on into row 0x22's first entry.
    charge_ticks = { 180, 60, 60, 60, 60, 60 },
    setup = taunt.setup,
}

return taunt
