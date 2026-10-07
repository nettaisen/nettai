--!strict
-- EXE4's cannons, the chips' action 0x0B (0x080EB854): on its first tick the navi shows animation 9, opens its counter
-- window (0x0800BA66: 16 ticks in a netbattle) and holds the cannon (an attachment, its animation the navi's number);
-- on the 15th the cannon's sound (0x8C) and its shot from the panel ahead, 24 pixels up, with the attack's damage word
-- (its +6 halfword added); after 29 ticks the recovery: animation 8 for 4 ticks, then the attack's exit (0x0800C9FC).
-- A GigaCan (variants 4 and up) leaves afterimages of the navi and its cannon on the 8th tick (and of a soul's part,
-- 0x08018068 by soul: the souls' to wire, docs/design/exe4-map.md §18 item 70). Each chip composes its own from its
-- variant's rows: the cannon (0x080EB9B4, an attachment row: Cannon's 0, HiCannon's 1, M-Cannon's 2, DrkCanon's 0x30,
-- the GigaCans' 0 to 2) and the shot (0x080EB9BB, a projectile row: 1, the GigaCans' 0x0C).

local attachment = require("../objects/attachment")
local projectile = require("./projectile")

local cannons = {}

local CANNON = asset.sprite("cannon")
-- The cannons (attachment rows 0, 1, 2 and 0x30: 0c-01 in palettes 0 to 3).
cannons.arm = attachment.look { sprite = CANNON, palette = 0 }
cannons.arm_hi = attachment.look { sprite = CANNON, palette = 1 }
cannons.arm_mega = attachment.look { sprite = CANNON, palette = 2 }
cannons.arm_dark = attachment.look { sprite = CANNON, palette = 3 }

export type CannonShot = {
    id: string,
    arm: AttachmentLook,
    shot: ProjectileVariant,
    -- A GigaCan's afterimages on the 8th tick.
    afterimages: boolean?,
}

type State = { timer: number }

local SHOOT, RECOVER = 0x0, 0x4
-- The shot's height, 16.16.
local SHOT_Z = 0x18_0000
-- The held cannon's animation: NaviStats +0x23, the navi's number (MegaMan's 0).
local NAVI_NUMBER = 0
-- The afterimages' color shader and lifetime.
local AFTERIMAGE_SHADER, AFTERIMAGE_TICKS = 0x8318, 0x1E
local CANNON_SOUND = asset.sound("cannon")

local function flip(me: Object): number
    return bit32.bxor(me.alliance, me.flip)
end

-- The chips' action 0x0B with a variant's rows.
function cannons.action(spec: CannonShot): Action
    -- 0x080EB874.
    local function shoot(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(9)
            me:open_counter_window()
            s.timer = 0
            attachment.attach(me, spec.arm, "related", NAVI_NUMBER)
            me:set_status("using_action", true)
            me.step_init = 4
            return
        end
        if s.timer == 8 and spec.afterimages then
            -- 0x080EB8BE: the navi in animation 9, then its cannon.
            battle.afterimage(me, me.pos, {
                anim = 9,
                flip = flip(me),
                color_shader = AFTERIMAGE_SHADER,
                lifetime = AFTERIMAGE_TICKS,
                shadow = "ground",
            })
            battle.afterimage(me, me.pos, {
                sprite = spec.arm.sprite,
                anim = NAVI_NUMBER,
                palette = spec.arm.palette,
                flip = flip(me),
                color_shader = AFTERIMAGE_SHADER,
                lifetime = AFTERIMAGE_TICKS,
                shadow = "with_sprite",
            })
        end
        if s.timer == 15 then
            battle.play_sound(CANNON_SOUND)
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            projectile.spawn(me, me.panel_x + me.facing, me.panel_y, { variant = spec.shot, damage = damage, z = SHOT_Z })
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

    return new.action {
        id = spec.id,
        state = { timer = "i16" },
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
end

return cannons
