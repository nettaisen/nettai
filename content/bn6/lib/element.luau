--!strict
-- Element bytes: an attack's primary element in the low bits and its
-- secondary elements above them, as objects and collision data keep them.
-- The pack's data names them (`element = "aqua"`, `secondary =
-- ["break"]`).

local element = {}

-- The primary elements' numbers, and the secondary elements' bits.
local PRIMARY: { [string]: number } = { null = 0, fire = 1, aqua = 2, elec = 3, wood = 4 }
local SECONDARY: { [string]: number } = { ["break"] = 0x10, wind = 0x20, cursor = 0x40, sword = 0x80 }

-- The element byte of `primary` with the `secondary` elements.
function element.byte(primary: string, secondary: { string }?): number
    local b = PRIMARY[primary]
    if b == nil then
        error(string.format("%q is not an element", primary))
    end
    for _, name in secondary or {} do
        local bit = SECONDARY[name]
        if bit == nil then
            error(string.format("%q is not a secondary element", name))
        end
        b = bit32.bor(b, bit)
    end
    return b
end

return element
