--!strict
-- Bass's shot (attack object #0x72, 0x080DB6FC: EXE4's own; EXE5's #0x72, its panel strike, is other code): on its
-- panel a pixel down, it highlights the panel for 10 ticks; then, on a solid panel, shows Bass's burst (his sheet's
-- animation 28, effect row 0x57) and hits the panel (single, the plain spark, a thrown hit of no element at navis,
-- modifier 3: a hitbox that runs while dimmed as the shot does) with its sound; and goes.
--
-- (Its Param1 0, which no spawner but EXE4's other Bass gives, waits 20 ticks with its highlight blinking and needs no
-- solid panel: 0x080DB740's ticks, 0x080DB744.)

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local shot = {}

local SOUND = asset.sound("vulcan")
-- Bass's burst (effect row 0x57: his sheet 08-19, animation 28).
local BURST = new.effect { sprite = asset.sprite("bass"), anim = 28, palette = 0 }
local SOLID = 0x10
local WARNING = 10

-- 0x080DB742: its panel highlighted; after its ticks, on a solid panel, the burst, the hit and its sound; then gone.
local function tick(me: Object)
    field.highlight(me.panel_x, me.panel_y)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    if bit32.btest(field.flags(me.panel_x, me.panel_y), SOLID) then
        battle.effect(me.pos, BURST, me.alliance)
        local box = battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = 0,
            z = 0,
            region = regions.single,
            hit_effect = sparks.plain,
            target = collision.hits_navis,
            self_type = collision.thrown,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = 3,
        })
        if box then
            box.run_while_dimmed = me.run_while_dimmed
        end
        battle.play_sound(SOUND)
    end
    me:set_lifecycle("destroy")
end

shot.kind = new.kind {
    id = "bass/shot",
    pool = "attack",
    -- 0x080DB6FC: by its state, then its sprite.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080DB720: on its panel, a pixel down; its ticks (Param1 1's, 0x080DB740).
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x, int.i32(p.y + 0x1_0000), p.z)
            me.timer = WARNING
            me:set_lifecycle("update")
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080DB7D2: a shot on (x, y) of Bass's side, of no element, with his damage word, running while dimmed. (Its
-- position is the spawner's registers: the row, the element 0 and 0.)
function shot.spawn(bass: Object, x: number, y: number, damage: number): Object?
    local o = battle.spawn(shot.kind, Vec3.new(y, 0, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = 0
    o:set_damage_word(damage)
    o.alliance = bass.alliance
    o.run_while_dimmed = true
    return o
end

return shot
