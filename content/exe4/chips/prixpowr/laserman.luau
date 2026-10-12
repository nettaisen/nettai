--!strict
-- PrixPowr's LaserMan (EXE4's actor object #0x54, 0x080CBCB0: EXE4's own, not the LaserMan chips' #0x4E), the second
-- of its three, on the panel ahead of the user's, where VideoMan projected his silhouette. He appears (his sound) and,
-- 21 ticks later, charges (animation 2); as that ends he fires his laser from the panel ahead along his row
-- (animation 3, its sound: chips/laserman/laser's course 5, the hit alone in half the time) and stands 51 ticks;
-- then, as his last animation (4) ends, he leaves his silhouette on his panel for 18 ticks (./silhouette) and goes,
-- with no puff. He runs while dimmed.

local laser = require("../laserman/laser")
local silhouette = require("./silhouette")

local laserman = {}

type State = {
    -- The controller waiting for him (the flag it gave him, his +0x4C).
    controller: Object?,
}

local SPRITE = asset.sprite("laserman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_FIRE = asset.sound("laser")
-- His laser's course (its first parameter, 0x080CBDBC).
local COURSE = 5

local APPEAR, ATTACK = 0x0, 0x4

-- Action 0 (0x080CBD54): 21 ticks; then his attack.
local function appear(me: Object)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        me:set_action(ATTACK)
    end
end

-- Action 4 (0x080CBD68): phase 0 (0x080CBD88) charging (animation 2; the 30 it sets its timer to nothing counts):
-- as the animation ends, the laser, animation 3, its sound; phase 4 (0x080CBDD2) 51 ticks, then animation 4; phase
-- 8 (0x080CBDE6) as it ends, his silhouette on his panel, and he goes.
local function attack(me: Object)
    if me.phase == 0 then
        if me.phase_init == 0 then
            me.phase_init = 1
            me.anim = 2
            me.timer = 30
            return
        end
        if not bit32.btest(me.sprite.frame_flags, 0x80) then
            return
        end
        me.anim = 3
        laser.spawn(me, me.panel_x + me.facing, me.panel_y, COURSE)
        me.timer = 50
        battle.play_sound(SOUND_FIRE)
        me.phase, me.phase_init = 4, 0
    elseif me.phase == 4 then
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left < 0 then
            me.anim = 4
            me.phase, me.phase_init = 8, 0
        end
    elseif me.phase == 8 then
        if bit32.btest(me.sprite.frame_flags, 0x80) then
            silhouette.show(me, me.panel_x, me.panel_y, 18)
            me:set_lifecycle("destroy")
        end
    else
        error(string.format("PrixPowr's LaserMan's attack's phase %#x reads past its table (0x080CBD7C)", me.phase))
    end
end

-- 0x080CBD34.
local function tick(me: Object)
    if me.action == APPEAR then
        appear(me)
    elseif me.action == ATTACK then
        attack(me)
    else
        error(string.format("PrixPowr's LaserMan's action %#x reads past his table (0x080CBD4C)", me.action))
    end
end

-- 0x080CBCE2: his sprite (08-16) with his shadow, shown, in animation 0, palette 0, facing his side's way, on his
-- panel; his sound; 20 ticks; then his first tick.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    battle.play_sound(SOUND_APPEAR)
    me.timer = 20
    me:set_lifecycle("update")
    tick(me)
end

laserman.kind = new.kind {
    id = "prixpowr/laserman",
    pool = "actor",
    state = { controller = "object" },
    -- 0x080CBCB0: by his state, then his sprite, while dimmed too.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x080CBCD4: the controller moves on; he is freed.
            local controller = (me.state :: State).controller
            if controller then
                navi_chip.navi_left(controller)
            end
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080CBE18: LaserMan on (x, y) for `controller`, of its side, with its element and the damage word. (His position
-- is the spawner's registers: the row, the element and 0.)
function laserman.summon(controller: Object, x: number, y: number, damage: number): Object?
    local o = battle.spawn(laserman.kind, Vec3.new(y, controller.element, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = controller.element
    o.alliance = controller.alliance
    o:set_damage_word(damage)
    ;(o.state :: State).controller = controller
    return o
end

return laserman
