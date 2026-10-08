--!strict
-- EXE4's row shot (attack object #0x6D, 0x080DAFBC) and the action that fires it (0x47, 0x080EDC5A): a shot that runs
-- along its row from the panel ahead until it hits something or leaves the field, with the status and sound its row
-- of 0x080DAFB0 gives (by its first parameter: rows 0 to 2 ThunderSoul's charged shot's, row 3 SandRing's, which
-- also leaves a pitfall where it hits and has no hand). Each user makes its own (`row_shot.make`): its kind, its row,
-- its look and the action's.

local attachment = require("./attachment")
local collision = require("../rules/collision")
local sparks = require("../lib/sparks")

local row_shot = {}

export type Spec = {
    -- Its kind's id and its action's.
    id: string,
    -- Its row of 0x080DAFB0: its speed (pixels a tick), the status its hits give (none: none) and its sound as it
    -- sets off.
    speed: number,
    status: Status?,
    sound: Sound,
    -- Its look (0c-37 for rows 0 to 2, 0c-3a for row 3) and its hit modifier (row 3's 1, else 0).
    sprite: SpriteAsset,
    hit_mod: number,
    -- The panel type its hit leaves (row 3's pitfall), if any.
    leaves: PanelType?,
    -- The action's animation (13; row 3's 18) and the hand it raises (attachment row 0x1D; row 3 none).
    anim: number,
    hand: AttachmentLook?,
}

-- Its height (the action's z, 26 pixels), and the navi's number (NaviStats +0x23: MegaMan's 0), the hand's animation.
local HEIGHT = 26 * 0x10000
local NAVI_NUMBER = 0
-- The action's ticks to the shot and to its end, and its recovery's.
local SHOT_AT, TICKS, RECOVER_TICKS = 5, 10, 10

type State = {
    -- AIAttackVars +0x10.
    timer: number,
}

function row_shot.make(spec: Spec): Action
    -- Gone: its region cleared, hidden, destroyed next tick.
    local function finish(me: Object)
        me.collision:set_region(nil)
        me.visible = false
        me:set_lifecycle("destroy")
    end

    -- 0x080DB06C: the hits against it resolve and spark; on a hit it ends (row 3 leaving its panel a pitfall);
    -- else, unless the battle is over, it runs on its row, ending off the field.
    local function tick(me: Object)
        local c = me.collision
        c:remove()
        me:hit_spark()
        if c.hit_flags ~= 0 then
            if spec.leaves then
                field.set_type(me.panel_x, me.panel_y, spec.leaves)
            end
            finish(me)
            return
        end
        if battle.over() then
            finish(me)
            return
        end
        local p = me.pos
        me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
        me:set_panel_from_coordinates()
        me:update_collision_panels()
        if not field.valid(me.panel_x, me.panel_y) then
            finish(me)
            return
        end
        c:present()
        me:update_sprite()
    end

    -- 0x080DAFDC: its look; on its panel's center at its height; forward at its row's speed; an attack at navis with
    -- its row's status and its element's spark; its row's sound.
    local function init(me: Object)
        me.visible = true
        local look = me.sprite
        look:load(spec.sprite)
        me.anim, me.anim_loaded = 0, 0
        look:set_animation(0)
        me:update_sprite()
        look.shadow = "with_sprite"
        local cx, cy = field.center(me.panel_x, me.panel_y)
        me.pos = Vec3.new(cx, cy, me.pos.z)
        me.vel = Vec3.new(me.facing * spec.speed * 0x10000, me.vel.y, me.vel.z)
        if not me:create_collision() then
            me:free()
            return
        end
        me:setup_collision(collision.attack, collision.hits_navis, spec.hit_mod)
        local c = me.collision
        if spec.status then
            c.status_base = spec.status
        end
        c:set_hit_effect(sparks.of_element(me.element))
        c:present()
        battle.play_sound(spec.sound)
        me:set_lifecycle("update")
    end

    local kind = new.kind {
        id = spec.id .. "/shot",
        pool = "attack",
        -- 0x080DAFBC.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
            end
        end,
    }

    -- 0x080DB0CC: the shot on (x, y), of `owner`'s side, with the attack's element and damage word (spawned at the
    -- caller's registers: X the panel's Y, Y the element, Z its height, which its init keeps).
    local function shoot(owner: Object, x: number, y: number, element: number, damage: number)
        local o = battle.spawn(kind, Vec3.new(y, element, HEIGHT))
        if o then
            o.panel_x, o.panel_y = x, y
            o.element = element
            o:set_damage_word(damage)
            o.alliance = owner.alliance
        end
    end


    -- 0x080EDC78: its animation, the counter window, the hand raised; ten ticks, on the fifth the hand's next frame
    -- and the shot from the panel ahead.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            me:set_status("using_action", true)
            me.anim, me.anim_loaded = spec.anim, 0xFF
            me:open_counter_window()
            if spec.hand then
                attachment.attach(me, spec.hand, "related", NAVI_NUMBER)
            end
            s.timer = TICKS
            me.step_init = 4
            return
        end
        if s.timer == SHOT_AT then
            local hand = me.related1
            if spec.hand and hand then
                hand.anim = 1
            end
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            shoot(me, me.panel_x + me.facing, me.panel_y, me.attack_element, damage)
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me.step, me.step_init = 4, 0
        end
    end

    -- 0x080EDD00: eleven ticks; then idle, the hand gone.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me:set_status("using_action", false)
            me.anim = 0
            me.related1 = nil
            me:exit_attack()
        end
    end

    return new.action {
        id = spec.id .. "/action",
        state = { timer = "u16" },
        -- 0x080EDC5A.
        update = function(me: Object, s: State)
            if me.step == 0 then
                fire(me, s)
            elseif me.step == 4 then
                recover(me, s)
            else
                error(string.format("the row shot's phase %#x reads past 0x080EDC70", me.step))
            end
        end,
    }
end

return row_shot
