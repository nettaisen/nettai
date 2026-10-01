--!strict
-- The engine's test content's own effects, hit sparks, region and collision
-- types for the ruleset's roles (rules/roles.luau): made-up looks (one
-- sprite for every effect, one for every spark), and collision types by
-- what they are.

local function effect(): Effect
    return define.effect { sprite = asset.sprite("test-burst"), anim = 0, palette = 0 }
end

local function spark(): Spark
    return define.spark { sprite = asset.sprite("test-spark"), anim = 0, palette = 0 }
end

-- A collision type's flag words: an attack, an object and a body of each
-- side, another body, a neutral object, a player's body, a floating body;
-- what resolves while the battle is dimmed, what reaches floating bodies.
local ATTACK = { [0] = 0x80000000, [1] = 0x40000000 }
local OBJECT = { [0] = 0x20000000, [1] = 0x10000000 }
local BODY = { [0] = 0x08000000, [1] = 0x04000000 }
local OTHER_BODY = { [0] = 0x02000000, [1] = 0x01000000 }
local NEUTRAL = 0x00800000
local PLAYER = { [0] = 0x00400000, [1] = 0x00200000 }
local FLOATING = 0x00100000
local WHILE_DIMMED = 0x00010000
local REACHES_FLOATING = 0x0080

-- A type whose flags `flags` gives for each side. `row` is the row the
-- type has in BN6's table: a bug code's high byte takes its offset.
local function collision(id: string, row: number, flags: (side: number) -> number): CollisionType
    return define.collision { id = "test/" .. id, side0 = flags(0), side1 = flags(1), row_offset = row * 8 }
end

local function body(side: number): number
    return bit32.bor(BODY[side], PLAYER[side], WHILE_DIMMED, REACHES_FLOATING)
end

local function attack(side: number): number
    return bit32.bor(ATTACK[side], REACHES_FLOATING)
end

-- The other side's objects and bodies, and neutral objects.
local function others(side: number): number
    local other = 1 - side
    return bit32.bor(OBJECT[other], BODY[other], OTHER_BODY[other], NEUTRAL)
end

return {
    effects = {
        deletion = effect(),
        recovery = effect(),
        cut_in_flash = effect(),
        trap_mark = effect(),
        encased = effect(),
        form_change = effect(),
        beast_over_gregar = effect(),
        beast_over_falzar = effect(),
        beast_over_blast = effect(),
        beast_over_burst = effect(),
    },
    sparks = {
        plain = spark(),
        guard = spark(),
        eruption = spark(),
        thrown_obstacle = spark(),
        uninstall = spark(),
    },
    regions = {
        anchor = define.region { panels = { { 0, 0 } } },
    },
    collision = {
        navi = collision("navi", 0x01, body),
        floating_navi = collision("floating-navi", 0x10, function(side)
            return bit32.bor(body(side), FLOATING)
        end),
        navi_target = collision("navi-target", 0x02, function(side)
            return bit32.bor(ATTACK[1 - side], others(side))
        end),
        eruption = collision("eruption", 0x48, attack),
        eruption_target = collision("eruption-target", 0x2A, others),
        thrown_obstacle = collision("thrown-obstacle", 0x06, attack),
        thrown_obstacle_target = collision("thrown-obstacle-target", 0x05, others),
    },
}
