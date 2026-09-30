--!strict
-- Set-up steps many attack objects share.

local object_setup = {}

-- `sub_8011504`: load `sprite` with a ground shadow (`ground`) or one that
-- follows the sprite's height, request animation `anim`, then give the
-- object collision of the types `self_type` / `target_type` with hit
-- modifier `hit_mod`, facing its way. False when no collision slot was
-- free (the sprite is loaded all the same).
function object_setup.sprite_and_collision(
    me: Object,
    sprite: SpriteId,
    anim: number,
    ground: boolean,
    self_type: number,
    target_type: number,
    hit_mod: number
): boolean
    local s = me.sprite
    s:load(sprite)
    s:set_animation(0)
    s.shadow = if ground then "ground" else "with_sprite"
    me.anim = anim
    me.anim_loaded = 0xFF
    if not me:create_collision() then
        return false
    end
    me:setup_collision(self_type, target_type, hit_mod)
    s:set_flip(bit32.bxor(me.alliance, me.flip))
    return true
end

-- `object_highlightCurrentCollisionPanels`: highlight the panels the
-- collision's region covers (drawn only).
function object_setup.highlight_collision_panels(me: Object)
    local c = me.collision
    local region = c.region
    local panels = data.regions[region]
    if panels == nil then
        error(string.format("region %#x is past PanelOffsetListsPointerTable", region))
    end
    local x, y, dir = c.panel_x, c.panel_y, me.facing
    for _, p in panels do
        field.highlight(x + dir * p[1], y + p[2])
    end
end

-- The dust puff an object leaves when it fails or bursts: effect 0x14, 16
-- pixels above it.
function object_setup.puff(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), 0x14)
end

return object_setup
