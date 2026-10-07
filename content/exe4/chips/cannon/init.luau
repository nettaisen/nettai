--!strict
-- Cannon (EXE4 chip 0x001; its record at 0x080197EC + 0x2C): EXE4's cannon, action 0x0B (0x080EB854), its variant 0
-- (the attack's +3). On its first tick the navi shows animation 9 with the cannon held (attachment row 0, 0c-01, its
-- animation the navi's number); on the 15th the cannon's sound (0x8C) and the cannon shot (projectile row 1) from the
-- panel in front, with the attack's damage word (its +6 halfword added: 0 for a chip, §18); after 29 ticks the recovery: animation 8 for 4 ticks,
-- then the attack's exit (0x0800C9FC). (Its variants 4 and up show afterimages on the 8th tick and fire row 0x0C:
-- HiCannon's and M-Cannon's to read, docs/design/exe4-map.md §18.)
--
-- Nearest the engine can say until read (exe4-map.md §18): its family (the record's +0x0B, 0x0B: EXE4's numbering is
-- to read), and what 0x0800BA66 does as it starts.

local attachment = require("../objects/attachment")
local projectile = require("../lib/projectile")
local variants = require("../objects/projectile/variants")

local SHOOT, RECOVER = 0x0, 0x4
local SHOT_Z = 0x18_0000
-- The held cannon's animation: NaviStats +0x23, the navi's number (MegaMan's 0).
local NAVI_NUMBER = 0

local held = attachment.look { sprite = asset.sprite("cannon"), palette = 0 }
-- The cannon's sound (0x8C).
local CANNON_SOUND = asset.sound("cannon")

type State = { timer: number }

-- 0x080EB874.
local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(9)
        s.timer = 0
        attachment.attach(me, held, "related", NAVI_NUMBER)
        me:set_status("using_action", true)
        me.step_init = 4
        return
    end
    if s.timer == 15 then
        battle.play_sound(CANNON_SOUND)
        local damage = bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16))
        projectile.spawn(me, me.panel_x + me.facing, me.panel_y, { variant = variants.cannon, damage = damage, z = SHOT_Z })
    end
    s.timer += 1
    if s.timer == 29 then
        me.step = RECOVER
        me.step_init = 0
    end
end

-- 0x080EB9C4.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.related1 = nil
        me:set_animation(8)
        s.timer = 3
        me.step_init = 4
    end
    s.timer -= 1
    if s.timer < 0 then
        me:exit_attack()
    end
end

local action = new.action {
    id = "cannon/action",
    state = { timer = "u16" },
    update = function(me: Object, s: State)
        local step = me.step
        if step == SHOOT then
            shoot(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("cannon phase %#x reads past its phase table", step))
        end
    end,
}

local cannon: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 8,
    damage = 40,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    action = action,
}

return { cannon = cannon }
