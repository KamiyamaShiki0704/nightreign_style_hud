#include <windows.h>
#include <iostream>
#include <unordered_map>
#include <vector>
#include <mutex>
#include <algorithm>
#include <string>
#include <MinHook.h>


namespace GameEngine {
    namespace Offsets {
       
        constexpr uintptr_t RVA_WORLDCHRMAN     = 0x3D65F88;
        constexpr uintptr_t RVA_GET_CHR_INS     = 0x507c80;
        constexpr uintptr_t RVA_DAMAGE_PROCESS  = 0x4483b0;
        constexpr uintptr_t RVA_APPLY_SPEFFECT  = 0x3E8BE0; 
        constexpr uintptr_t RVA_INTERNAL_SPEFFECT_HOOK = 0x4FA8E0; 
        constexpr uintptr_t RVA_GAME_DATA_MAN   = 0xD5DF38;

        
        constexpr ptrdiff_t WCM_LOCAL_PLAYER    = 0x1E508;
        constexpr ptrdiff_t CHR_TARGET_HANDLE   = 0x6B0;
        constexpr ptrdiff_t CHR_MODULE_BAG      = 0x190;
        constexpr ptrdiff_t MODULE_DAMAGE       = 0x98;
    }

    inline uintptr_t GetBaseAddress() {
        static uintptr_t base = reinterpret_cast<uintptr_t>(GetModuleHandleW(L"eldenring.exe"));
        return base;
    }
    inline uintptr_t GetWorldChrMan() {
        return *reinterpret_cast<uintptr_t*>(GetBaseAddress() + Offsets::RVA_WORLDCHRMAN);
    }
    inline uintptr_t GetLocalPlayer() {
        uintptr_t wcm = GetWorldChrMan();
        if (!wcm) return 0;
        return *reinterpret_cast<uintptr_t*>(wcm + Offsets::WCM_LOCAL_PLAYER);
    }
    inline uint64_t GetLockedTargetHandle(uintptr_t localPlayer) {
        if (!localPlayer) return 0;
        return *reinterpret_cast<uint64_t*>(localPlayer + Offsets::CHR_TARGET_HANDLE);
    }
    inline uintptr_t GetChrInsFromHandle(uint64_t* handlePtr) {
        if (!handlePtr || *handlePtr == 0 || *handlePtr == 0xFFFFFFFFFFFFFFFF) return 0;
        uintptr_t wcm = GetWorldChrMan();
        if (!wcm) return 0;

        typedef void* (__fastcall* tGetChrInsFromHandle)(void*, uint64_t*);
        auto fnGetChrIns = reinterpret_cast<tGetChrInsFromHandle>(GetBaseAddress() + Offsets::RVA_GET_CHR_INS);
        return reinterpret_cast<uintptr_t>(fnGetChrIns(reinterpret_cast<void*>(wcm), handlePtr));
    }
    inline uintptr_t GetDamageModuleFromChrIns(uintptr_t chrIns) {
        if (!chrIns) return 0;
        uintptr_t moduleBag = *reinterpret_cast<uintptr_t*>(chrIns + Offsets::CHR_MODULE_BAG);
        if (!moduleBag) return 0;
        return *reinterpret_cast<uintptr_t*>(moduleBag + Offsets::MODULE_DAMAGE);
    }
    inline uintptr_t GetMagicMemoryBase() {
        uintptr_t gdm = *reinterpret_cast<uintptr_t*>(GetBaseAddress() + Offsets::RVA_GAME_DATA_MAN);
        if (!gdm) return 0;
        uintptr_t pgd = *reinterpret_cast<uintptr_t*>(gdm + 0x08);
        if (!pgd) return 0;
        return *reinterpret_cast<uintptr_t*>(pgd + 0x530);
    }

    inline void CallOriginalApplySpEffect(uintptr_t chrIns, uint32_t spEffectId, bool dontSync = true) {
        if (!chrIns || spEffectId == 0) return;
        typedef uint64_t(__fastcall* tApplySpEffect)(uintptr_t, int32_t, bool);
        auto fnApply = reinterpret_cast<tApplySpEffect>(GetBaseAddress() + Offsets::RVA_APPLY_SPEFFECT);
        fnApply(chrIns, static_cast<int32_t>(spEffectId), dontSync);
    }
}

enum class ElementType : int {
    None = 0,
    Magic = 1,      // r8 + 0x238
    Fire = 2,       // r8 + 0x234
    Lightning = 3,  // r8 + 0x23c
    Holy = 4        // r8 + 0x240
};

std::string ElementToString(ElementType type) {
    switch (type) {
        case ElementType::Magic:     return "Magic (魔)";
        case ElementType::Fire:      return "Fire (火)";
        case ElementType::Lightning: return "Lightning (雷)";
        case ElementType::Holy:      return "Holy (圣)";
        default:                     return "None (无)";
    }
}


class HermitSpellManager {
private:
    std::mutex mtx;
    uintptr_t lastPlayerAddress = 0; // 缓存上一次按键时的玩家实例基址

public:
    std::unordered_map<uintptr_t, ElementType> enemyLastDamageMap;
    std::vector<ElementType> elementQueue;
    bool isSpellReady = false;
    bool pendingSynthesis = false;  
    int savedMagicSlot = -1;
    int savedOriginalMagicID = 0;

    static HermitSpellManager& Get() {
        static HermitSpellManager instance;
        return instance;
    }

 
    int CalculateCombinedSpellID() {
        if (elementQueue.size() < 3) return 0;
        std::vector<int> sortedElements = { (int)elementQueue[0], (int)elementQueue[1], (int)elementQueue[2] };
        std::sort(sortedElements.begin(), sortedElements.end());
        int noOrderScore = sortedElements[0] * 100 + sortedElements[1] * 10 + sortedElements[2];
        
        switch (noOrderScore) {
            case 111: return 70010000; // 3魔
            case 222: return 70020000; // 3火
            case 112: return 70030000; // 2魔1火
            case 122: return 70040000; // 1魔2火
            case 123: return 70050000; // 1魔1火1雷
            default:  return 70000000; // 默认大魔法
        }
    }

    void RecordDamage(uintptr_t enemyDmgModule, ElementType element) {
        std::lock_guard<std::mutex> lock(mtx);
        if (element != ElementType::None) {
            enemyLastDamageMap[enemyDmgModule] = element;
        }
    }

    void TryAcquireElement(uintptr_t lockedEnemyIns) {
        uintptr_t currentPlayerAddress = GameEngine::GetLocalPlayer();
        
        if (lastPlayerAddress != 0 && currentPlayerAddress != lastPlayerAddress) {
            std::lock_guard<std::mutex> lock(mtx);
            enemyLastDamageMap.clear();  
            elementQueue.clear();        
            isSpellReady = false;
            pendingSynthesis = false;
            std::cout << "[生命周期净化] 检测到玩家实例地址已重置（死亡/坐火），已自动清空 Map 与旧队列！" << std::endl;
        }   
        lastPlayerAddress = currentPlayerAddress;

        std::lock_guard<std::mutex> lock(mtx);       
        if (isSpellReady || elementQueue.size() >= 3) return;
        if (!lockedEnemyIns) return;

        
        uintptr_t enemyDmgModule = GameEngine::GetDamageModuleFromChrIns(lockedEnemyIns);
        if (!enemyDmgModule) return;

        auto it = enemyLastDamageMap.find(enemyDmgModule);
        if (it != enemyLastDamageMap.end() && it->second != ElementType::None) {
            ElementType acqElement = it->second;
            elementQueue.push_back(acqElement);
            
            std::cout << "\n[+] 成功吸收元素 -> " << ElementToString(acqElement) << std::endl;
            if (currentPlayerAddress) {
                uint32_t debugSpEffectId = 4330 ;// 基础标识ID + 元素数值
                GameEngine::CallOriginalApplySpEffect(currentPlayerAddress, debugSpEffectId, true);
               
                std::cout << "[SpEffect挂载] 成功向本地玩家附加标识特效 SpEffect ID: " << debugSpEffectId << std::endl;
            }

            PrintQueueAndMap();
            
            if (elementQueue.size() == 3) {
                pendingSynthesis = true;
                std::cout << "[!] 队列已集满 3 个元素，向主线程挂起安全同步覆写请求..." << std::endl;
            }
        } else {
            std::cout << "[-] 无法吸取：当前锁定的目标没有任何属性伤害记录！" << std::endl;
        }
    }

 
    void SafeProcessSynthesis() {
        std::lock_guard<std::mutex> lock(mtx);
        if (!pendingSynthesis) return;

        uintptr_t magicBase = GameEngine::GetMagicMemoryBase();
        if (!magicBase) return;

        int currentSlot = *reinterpret_cast<int*>(magicBase + 0x80);
        savedMagicSlot = currentSlot;
        savedOriginalMagicID = *reinterpret_cast<int*>(magicBase + 0x10 + (currentSlot * 8));

        int combinedSpellID = CalculateCombinedSpellID();
        //强制修改当前魔法槽位 
        // *reinterpret_cast<int*>(magicBase + 0x10 + (currentSlot * 8)) = combinedSpellID;  

        pendingSynthesis = false;
        isSpellReady = true;
        std::cout << "组合法术生成完毕！已锁定当前槽位 " << currentSlot << " 覆写复合 ID: " << combinedSpellID << std::endl;
    }

    void OnSpellConsumed() {
        std::lock_guard<std::mutex> lock(mtx);
        if (!isSpellReady) return;

        uintptr_t magicBase = GameEngine::GetMagicMemoryBase();
        if (magicBase && savedMagicSlot != -1) {
            *reinterpret_cast<int*>(magicBase + 0x10 + (savedMagicSlot * 8)) = savedOriginalMagicID;
            std::cout << "组合魔法已释放。原始魔法 ID: " << savedOriginalMagicID << " 已被强制写回锁定的旧槽位: " << savedMagicSlot << std::endl;
        }

        elementQueue.clear();
        isSpellReady = false;
        savedMagicSlot = -1;
        savedOriginalMagicID = 0;
        std::cout << "系统全部重置\n" << std::endl;
    }

    void PrintQueueAndMap() {
        std::cout << "============ [ 隐士系统状态面板 ] ============" << std::endl;
        std::cout << "当前吸收队列 [" << elementQueue.size() << "/3]: ";
        if (elementQueue.empty()) std::cout << "(空)";
        for (size_t i = 0; i < elementQueue.size(); ++i) {
            std::cout << "[" << i + 1 << "]:" << ElementToString(elementQueue[i]) << "  ";
        }
        std::cout << std::endl;

        std::cout << "伤害字典注册表 ( enemyLastDamageMap 共有 " << enemyLastDamageMap.size() << " 个实体记录 ):" << std::endl;
        int count = 0;
        for (const auto& [key, value] : enemyLastDamageMap) {
            std::cout << "  -> Key [DamageModule]: 0x" << std::hex << key 
                      << std::dec << " | Value [Element]: " << ElementToString(value) << std::endl;
            if (++count >= 10) {
                std::cout << "     ... (其余记录已省略)" << std::endl;
                break;
            }
        }
        std::cout << "===============================================" << std::endl;
    }
};


typedef void* (__fastcall* tDamageProcess)(uintptr_t rcx, uintptr_t rdx, uintptr_t r8, uintptr_t r9);
tDamageProcess opDamageProcess = nullptr;

void* __fastcall hkDamageProcess(uintptr_t rcx, uintptr_t rdx, uintptr_t r8, uintptr_t r9) {
    if (HermitSpellManager::Get().pendingSynthesis) {
        HermitSpellManager::Get().SafeProcessSynthesis();
    }

    if (rcx && r8) {
        int32_t fire    = *reinterpret_cast<int32_t*>(r8 + 0x234);
        int32_t magic   = *reinterpret_cast<int32_t*>(r8 + 0x238);
        int32_t thunder = *reinterpret_cast<int32_t*>(r8 + 0x23c);
        int32_t holy    = *reinterpret_cast<int32_t*>(r8 + 0x240);

        ElementType currentElement = ElementType::None;
        int32_t maxDmg = 0;

        if (magic > maxDmg)   { maxDmg = magic;   currentElement = ElementType::Magic; }
        if (fire > maxDmg)    { maxDmg = fire;    currentElement = ElementType::Fire; }
        if (thunder > maxDmg) { maxDmg = thunder; currentElement = ElementType::Lightning; }
        if (holy > maxDmg)    { maxDmg = holy;    currentElement = ElementType::Holy; }

        if (currentElement != ElementType::None) {
            HermitSpellManager::Get().RecordDamage(rcx, currentElement);
        }
    }
    return opDamageProcess(rcx, rdx, r8, r9);
}

typedef uintptr_t(__fastcall* tSpecialEffectApply)(void* pSpecialEffect, int32_t effect_id, uintptr_t p3, uintptr_t p4, uintptr_t p5, uintptr_t p6, uintptr_t p7, uintptr_t p8);
tSpecialEffectApply opSpecialEffectApply = nullptr;

uintptr_t __fastcall Hooked_SpecialEffectApply(void* pSpecialEffect, int32_t effect_id, uintptr_t p3, uintptr_t p4, uintptr_t p5, uintptr_t p6, uintptr_t p7, uintptr_t p8) {
    if (effect_id == 1234567) {
        HermitSpellManager::Get().OnSpellConsumed();
    }
    return opSpecialEffectApply(pSpecialEffect, effect_id, p3, p4, p5, p6, p7, p8);
}



DWORD WINAPI InputMonitorThread(LPVOID lpParam) {
    std::cout << "[+] Tab键元素吸取轮询线程启动成功。" << std::endl;

    while (true) {
        // 监听 [Tab] 按键按下
        if (GetAsyncKeyState(VK_TAB) & 0x8000) {
            // 使用封装好的解耦函数层层寻址获取锁定目标 ChrIns
            uintptr_t localPlayer = GameEngine::GetLocalPlayer();
            if (localPlayer) {
                uint64_t targetHandle = GameEngine::GetLockedTargetHandle(localPlayer);
                
                if (targetHandle != 0 && targetHandle != 0xFFFFFFFFFFFFFFFF) {
                    uintptr_t targetChrIns = GameEngine::GetChrInsFromHandle(&targetHandle);
                    if (targetChrIns) {
                        // 提交给单例处理
                        HermitSpellManager::Get().TryAcquireElement(targetChrIns);
                    }
                } else {
                    std::cout << "[-] 无法吸取：当前视野内未锁定任何目标！" << std::endl;
                }
            }
            Sleep(250); // 防抖
        }
        Sleep(10);
    }
    return 0;
}


void MainLogic(HMODULE hModule) {
    AllocConsole();
    FILE* fDummy;
    freopen_s(&fDummy, "CONOUT$", "w", stdout);
    freopen_s(&fDummy, "CONIN$", "r", stdin);

    std::cout << "=====================================================" << std::endl;
    std::cout << "   Elden Ring 隐士多元素同步重构优化版 (完全解耦)    " << std::endl;
    std::cout << "=====================================================" << std::endl;

    if (MH_Initialize() != MH_OK) {
        std::cout << "[-] MinHook 初始化失败！" << std::endl;
        return;
    }

    uintptr_t base = GameEngine::GetBaseAddress();

    // 从解耦命名空间中调取偏移并挂钩
    MH_CreateHook(reinterpret_cast<LPVOID>(base + GameEngine::Offsets::RVA_DAMAGE_PROCESS), &hkDamageProcess, reinterpret_cast<LPVOID*>(&opDamageProcess));
    MH_CreateHook(reinterpret_cast<LPVOID>(base + GameEngine::Offsets::RVA_INTERNAL_SPEFFECT_HOOK), &Hooked_SpecialEffectApply, reinterpret_cast<LPVOID*>(&opSpecialEffectApply));
    
    MH_EnableHook(MH_ALL_HOOKS);
    std::cout << "[+] 底层 Hook 代理同步中枢与事件监听中枢全部拦截挂载就绪。" << std::endl;

    // 创建按键监控线程
    CreateThread(nullptr, 0, &InputMonitorThread, nullptr, 0, nullptr);
}

BOOL APIENTRY DllMain(HMODULE hModule, DWORD ul_reason_for_call, LPVOID lpReserved) {
    switch (ul_reason_for_call) {
        case DLL_PROCESS_ATTACH:
            DisableThreadLibraryCalls(hModule);
            CreateThread(nullptr, 0, reinterpret_cast<LPTHREAD_START_ROUTINE>(MainLogic), hModule, 0, nullptr);
            break;
        case DLL_PROCESS_DETACH:
            MH_DisableHook(MH_ALL_HOOKS);
            MH_Uninitialize();
            FreeConsole();
            break;
    }
    return TRUE;
}