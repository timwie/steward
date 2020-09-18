# gbx

## XML-RPC Reference

##### Legend
```
[ ] not in use yet
[x] in use
[/] won't need it
[?] unsure if or how it works
[!] broken, deprecated, or has a better alternative
```

### XML-RPC Methods
As of server API version `2013-04-16`:
```
[/] system.listMethods
[/] system.methodSignature
[/] system.methodHelp
[/] system.multicall

[x] Authenticate
[/] ChangeAuthPassword
[x] EnableCallbacks
[x] SetApiVersion
[x] GetVersion
[/] GetStatus
[x] QuitGame

[x] ChatSendServerMessage
[/] ChatSendServerMessageToLanguage
[x] ChatSendServerMessageToId/Login
[/] ChatSend
[/] ChatSendToLanguage
[/] ChatSendToLogin/Id
[/] GetChatLines
[x] ChatEnableManualRouting
[x] ChatForwardToLogin

[x] SendDisplayManialinkPage
[x] SendDisplayManialinkPageToId/Login
[x] SendHideManialinkPage
[/] SendHideManialinkPageToId/Login

[x] Kick(Id)
[x] BlackList(Id)
[x] UnBlackList
[ ] CleanBlackList
[x] GetBlackList
[x] LoadBlackList
[x] SaveBlackList

[ ] AddGuest(Id)  // allowed to enter without free player slot or password
[ ] RemoveGuest(Id)
[ ] CleanGuestList
[ ] GetGuestList
[ ] LoadGuestList
[ ] SaveGuestList

[ ] Echo  // trigger echo callback on other controllers

[ ] Ignore(Id)  // players on the ignore list are muted
[ ] UnIgnore(Id)
[ ] CleanIgnoreList
[ ] GetIgnoreList

[x] SetServerOptions
[x] GetServerOptions

[x] GameDataDirectory
[/] GetMapsDirectory
[/] GetSkinsDirectory

[/] GetModeScriptText
[/] SetModeScriptText
[x] GetModeScriptInfo
[x] GetModeScriptSettings
[x] SetModeScriptSettings
[ ] SendModeScriptCommands  // for #Command directives in mode scripts
[/] SetModeScriptSettingsAndCommands
[x] TriggerModeScriptEvent
[x] TriggerModeScriptEventArray

[x] RestartMap
[x] NextMap

[x] SetScriptName
[/] GetScriptName

[x] GetCurrentMapIndex
[x] GetNextMapIndex
[x] SetNextMapIndex/Ident
[ ] JumpToMapIndex/Ident

[/] GetCurrentMapInfo
[/] GetNextMapInfo
[x] GetMapInfo

[x] GetMapList
[x] AddMap
[x] AddMapList
[x] RemoveMap
[x] RemoveMapList
[ ] InsertMap
[ ] InsertMapList
[ ] ChooseNextMap
[ ] ChooseNextMapList

[ ] LoadMatchSettings
[x] SaveMatchSettings
[ ] AppendPlaylistFromMatchSettings
[ ] InsertPlaylistFromMatchSettings

[x] GetPlayerList
[/] GetPlayerInfo
[/] GetDetailedPlayerInfo
[/] GetMainServerPlayerInfo

[x] ForceSpectator(Id)
[ ] ForceSpectatorTarget(Id)
[x] SpectatorReleasePlayerSlot(Id)

[x] GetNetworkStats

[/] StartServerLan
[ ] StartServerInternet  // there might be a use case for stopping/starting servers, without shutting down the server program
[ ] StopServer

[/] GetSystemInfo  // pretty much "GetConnectionRates"; not much of interest here
[/] SetConnectionRates  // doesn't seem like something you'd need to change on the fly

[/] CallVote  // not planning to bother with these
[/] CallVoteEx
[/] InternalCallVote
[/] CancelVote
[/] GetCurrentCallVote
[/] SetCallVoteTimeOut
[/] GetCallVoteTimeOut
[/] SetCallVoteRatio
[/] GetCallVoteRatio
[/] SetCallVoteRatios
[/] GetCallVoteRatios
[/] SetCallVoteRatiosEx
[/] GetCallVoteRatiosEx

[/] Ban(Id)  // compared to the blacklist, the banlist is apparently temporary; not sure why we'd use it
[/] BanAndBlackList
[/] UnBan
[/] CleanBanList
[/] GetBanList

[/] SetChatTime  // not needed since we have Get/SetModeScriptSettings 
[/] GetChatTime
[/] SetAllWarmUpDuration
[/] GetAllWarmUpDuration
[/] SetDisableRespawn
[/] GetDisableRespawn

[/] SetServerName  // not needed since we have Get/SetServerOptions
[/] GetServerName
[/] SetServerComment
[/] GetServerComment
[/] SetHideServer
[/] GetHideServer
[/] SetServerPassword
[/] GetServerPassword
[/] SetServerPasswordForSpectator
[/] GetServerPasswordForSpectator
[/] SetMaxPlayers
[/] GetMaxPlayers
[/] SetMaxSpectators
[/] GetMaxSpectators
[/] KeepPlayerSlots
[/] IsKeepingPlayerSlots
[/] EnableP2PUpload
[/] IsP2PUpload
[/] EnableP2PDownload
[/] IsP2PDownload
[/] AllowMapDownload
[/] IsMapDownloadAllowed
[/] DisableHorns
[/] AreHornsDisabled
[/] DisableServiceAnnounces
[/] AreServiceAnnouncesDisabled
[/] SetLadderMode
[/] GetLadderMode
[/] SetVehicleNetQuality
[/] GetVehicleNetQuality
[/] AutoSaveReplays
[/] AutoSaveValidationReplays
[/] IsAutoSaveReplaysEnabled
[/] IsAutoSaveValidationReplaysEnabled
[/] SetRefereePassword
[/] GetRefereePassword
[/] SetRefereeMode
[/] GetRefereeMode
[/] SetUseChangingValidationSeed
[/] GetUseChangingValidationSeed
[/] SetClientInputsMaxLatency
[/] GetClientInputsMaxLatency

[/] SetForcedMods  // not a priority to support these
[/] GetForcedMods
[/] SetForcedMusic
[/] GetForcedMusic

[?] SetForceShowAllOpponents  // unsure if these work
[?] GetForceShowAllOpponents

[?] SetFinishTimeout  // unsure if these do anything in Rounds/Laps mode
[?] GetFinishTimeout

[?] CheckEndMatchCondition  // haven't found any further documentation for these
[?] CheckMapForCurrentServerParams
[?] GetLastConnectionErrorMessage
[?] GetManialinkPageAnswers

[?] AutoTeamBalance  // unsure if these do anything in Teams mode
[?] SetTeamInfo  
[?] GetTeamInfo
[?] ForcePlayerTeam(Id)
[?] SetForcedTeams
[?] GetForcedTeams

[?] SendOpenLinkToId/Login  // no manialinks in TMNext; unsure whether you can send server join links

[?] SendNotice  // haven't tried these out
[?] SendNoticeToId/Login

[?] GetModeScriptVariables  // empty in TimeAttack at least
[?] SetModeScriptVariables

[?] CustomizeQuitDialog  // not sure these work in TMNext
[?] SendToServerAfterMatchEnd

[?] ConnectFakePlayer  // not sure how useful this is
[?] DisconnectFakePlayer

[?] SetTimeAttackSynchStartPeriod  // haven't found any further documentation for these
[?] GetTimeAttackSynchStartPeriod

[?] SetServerPlugin  // don't know if there are any plugins we'd want to interact with
[?] GetServerPlugin
[?] GetServerPluginVariables
[?] SetServerPluginVariables
[?] TriggerServerPluginEvent
[?] TriggerServerPluginEventArray

[?] GetScriptCloudVariables  // haven't found any further documentation for these
[?] SetScriptCloudVariables

[?] WriteFile  // don't know why you would send map files to clients?

[!] SaveCurrentReplay
[!] SaveBestGhostsReplay  // broke with TMNext
[!] GetValidationReplay   // broke with TMNext

[!] SetGameMode  // for old, hardcoded Trackmania modes
[!] GetGameMode
[!] SetWarmUp
[!] GetWarmUp

[!] GetCurrentRanking(ForLogin)  // plenty of other ways to keep track of scores
[!] GetCurrentWinnerTeam

[!] SetGameInfos  // these seem very much outdated
[!] GetCurrentGameInfo
[!] GetNextGameInfo
[!] GetGameInfos

[!] SetTimeAttackLimit  // I assume these are for old, hardcoded Trackmania modes
[!] GetTimeAttackLimit
[!] SetLapsTimeLimit
[!] GetLapsTimeLimit
[!] SetNbLaps
[!] GetNbLaps
[!] SetRoundForcedLaps
[!] GetRoundForcedLaps
[!] SetRoundPointsLimit
[!] GetRoundPointsLimit
[!] SetRoundCustomPoints
[!] GetRoundCustomPoints
[!] SetUseNewRulesRound
[!] GetUseNewRulesRound
[!] SetTeamPointsLimit
[!] GetTeamPointsLimit
[!] SetMaxPointsTeam
[!] GetMaxPointsTeam
[!] SetUseNewRulesTeam
[!] GetUseNewRulesTeam
[!] SetCupPointsLimit
[!] GetCupPointsLimit
[!] SetCupRoundsPerMap
[!] GetCupRoundsPerMap
[!] SetCupWarmUpDuration
[!] GetCupWarmUpDuration
[!] SetCupNbWinners
[!] GetCupNbWinners

[!] ForceScores  // probably deprecated, and there's a script method for it
[!] ForceEndRound

[!] SetForcedSkins  // only club skins in TMNext
[!] GetForcedSkins

[!] Pay  // for ManiaPlanet payments
[!] SendBill
[!] GetBillState
[!] GetServerPlanets

[!] SetBuddyNotification  // I assume these no longer work
[!] GetBuddyNotification

[!] GetServerTags  // I doubt these are still a thing
[!] SetServerTag
[!] UnsetServerTag
[!] ResetServerTags

[!] SetForcedClubLinks   // I doubt these are still a thing
[!] GetForcedClubLinks

[!] SetLobbyInfo  // not sure if there'll be lobbies n TMNext
[!] GetLobbyInfo

[!] GetDemoTokenInfosForPlayer  // sounds like something deprecated

[!] GetLadderServerLimits  // no limits in TMNext

[!] IsRelayServer  // not sure if relay servers are still a thing

[!] TunnelSendDataToId  // apparently related to relay servers
[!] TunnelSendDataToLogin

[!] ManualFlowControlEnable  // don't bother testing these
[!] ManualFlowControlProceed
[!] ManualFlowControlIsEnabled
[!] ManualFlowControlGetCurTransition
```

### XML-RPC Callbacks
As of server API version `2013-04-16`:
```
[x] ManiaPlanet.ModeScriptCallback
[x] ManiaPlanet.ModeScriptCallbackArray

[ ] ManiaPlanet.Echo  // get echoes from other controllers

[ ] ManiaPlanet.MapListModified  // usually the controller will make the modification; but might add to make sure we're always in sync

[/] ManiaPlanet.PlayerConnect
[x] ManiaPlanet.PlayerDisconnect
[x] ManiaPlanet.PlayerInfoChanged
[x] TrackMania.PlayerIncoherence

[x] ManiaPlanet.PlayerChat
[x] ManiaPlanet.PlayerManialinkPageAnswer

[/] ManiaPlanet.VoteUpdated

[/] ManiaPlanet.BeginMap  // all of these can be replaced with script callbacks
[/] ManiaPlanet.BeginMatch
[/] ManiaPlanet.EndMap
[/] ManiaPlanet.EndMatch
[/] ManiaPlanet.ServerStart
[/] ManiaPlanet.ServerStop
[/] ManiaPlanet.StatusChanged
[/] TrackMania.PlayerCheckpoint
[/] TrackMania.PlayerFinish

[!] ManiaPlanet.BillUpdated  // ManiaPlanet payments

[!] ManiaPlanet.PlayerAlliesChanged  // might be related to lobbies

[!] ManiaPlanet.TunnelDataReceived  // apparently related to relay servers

[?] ScriptCloud.LoadData  // haven't found any documentation for these
[?] ScriptCloud.SaveData
```

### XML-RPC Script Methods
As of script API version `3.1.0`:

```
[x] XmlRpc.EnableCallbacks
[/] XmlRpc.GetCallbacksList
[/] XmlRpc.GetCallbacksList_Enabled
[/] XmlRpc.GetCallbacksList_Disabled
[/] XmlRpc.BlockCallbacks
[/] XmlRpc.UnblockCallbacks
[/] XmlRpc.GetCallbackHelp
[/] XmlRpc.GetMethodsList
[/] XmlRpc.GetMethodHelp
[/] XmlRpc.GetDocumentation
[x] XmlRpc.SetApiVersion
[/] XmlRpc.GetApiVersion
[/] XmlRpc.GetAllApiVersions

[ ] Trackmania.SetPlayerPoints
[ ] Trackmania.SetTeamPoints

[x] Trackmania.WarmUp.ForceStop
[/] Trackmania.WarmUp.ForceStopRound
[x] Trackmania.WarmUp.Extend
[x] Trackmania.WarmUp.GetStatus

[x] Maniaplanet.Pause.GetStatus
[x] Maniaplanet.Pause.SetActive

[x] Trackmania.GetScores

[/] Maniaplanet.Mode.GetUseTeams

[/] Trackmania.Event.SetCurRaceCheckpointsMode  // don't see the use for these
[/] Trackmania.Event.SetCurLapCheckpointsMode
[/] Trackmania.Event.UnsetCurRaceCheckpointsMode
[/] Trackmania.Event.UnsetCurLapCheckpointsMode

[/] Maniaplanet.UI.SetAltScoresTableVisibility  // can be done in ManiaScript
[/] Maniaplanet.UI.SetScoresTableVisibility

[!] Maniaplanet.WarmUp.GetStatus  // use Trackmania.WarmUp.GetStatus
```

### XML-RPC Script Callbacks
As of script API version `3.1.0`:

```
[/] XmlRpc.CallbacksList
[/] XmlRpc.CallbacksList_Enabled
[/] XmlRpc.CallbacksList_Disabled
[/] XmlRpc.CallbackHelp
[/] XmlRpc.MethodsList
[/] XmlRpc.MethodHelp
[/] XmlRpc.Documentation
[/] XmlRpc.ApiVersion
[/] XmlRpc.AllApiVersions

[x] Maniaplanet.StartServer_Start
[x] Maniaplanet.StartServer_End
[x] Maniaplanet.StartMatch_Start
[x] Maniaplanet.StartMatch_End
[x] Maniaplanet.LoadingMap_Start
[x] Maniaplanet.LoadingMap_End
[x] Maniaplanet.StartMap_Start
[x] Maniaplanet.StartMap_End
[x] Maniaplanet.StartRound_Start
[x] Maniaplanet.StartRound_End
[?] Maniaplanet.StartTurn_Start  // does any mode use turns?
[?] Maniaplanet.StartTurn_End
[x] Maniaplanet.StartPlayLoop
[x] Maniaplanet.EndPlayLoop 
[?] Maniaplanet.EndTurn_Start  // does any mode use turns?
[?] Maniaplanet.EndTurn_End
[x] Maniaplanet.EndRound_Start
[x] Maniaplanet.EndRound_End
[x] Maniaplanet.EndMap_Start
[x] Maniaplanet.EndMap_End
[x] Maniaplanet.UnloadingMap_Start
[x] Maniaplanet.UnloadingMap_End
[x] Maniaplanet.EndMatch_Start
[x] Maniaplanet.EndMatch_End
[x] Maniaplanet.EndServer_Start
[x] Maniaplanet.EndServer_End

[/] Trackmania.Event.Default
[ ] Trackmania.Event.OnCommand  // for #Command directives in mode scripts I assume
[/] Trackmania.Event.OnPlayerAdded
[/] Trackmania.Event.OnPlayerRemoved
[x] Trackmania.Event.StartLine
[x] Trackmania.Event.WayPoint
[x] Trackmania.Event.GiveUp
[x] Trackmania.Event.SkipOutro
[ ] Trackmania.Event.Respawn

[ ] Trackmania.WarmUp.Start
[x] Trackmania.WarmUp.StartRound
[x] Trackmania.WarmUp.EndRound
[ ] Trackmania.WarmUp.End
[x] Trackmania.WarmUp.Status

[x] Maniaplanet.Pause.Status

[x] Trackmania.Scores

[/] Maniaplanet.Mode.UseTeams

[/] UI.Event.Default
[?] UI.Event.OnModuleCustomEvent  // don't know anything about modules
[?] UI.Event.OnModuleShowRequest
[?] UI.Event.OnModuleHideRequest
[?] UI.Event.OnModuleStorePurchase
[?] UI.Event.OnModuleInventoryDrop
[?] UI.Event.OnModuleInventoryEquip

[!] Maniaplanet.WarmUp.Status  // use Trackmania.WarmUp.Status

[!] Maniaplanet.ChannelProgression_Start  // channels are Nadeo servers only
[!] Maniaplanet.ChannelProgression_End

[!] Maniaplanet.Podium_Start  // there's no podiums in TMNext yet
[!] Maniaplanet.Podium_End

[!] Trackmania.Event.OnShoot  // leftovers from Shootmania
[!] Trackmania.Event.OnHit
[!] Trackmania.Event.OnNearMiss
[!] Trackmania.Event.OnArmorEmpty
[!] Trackmania.Event.OnCapture
[!] Trackmania.Event.OnShotDeny
[!] Trackmania.Event.OnFallDamage
[!] Trackmania.Event.OnPlayerRequestRespawn
[!] Trackmania.Event.OnActionCustomEvent
[!] Trackmania.Event.OnActionEvent
[!] Trackmania.Event.OnPlayerTouchesObject
[!] Trackmania.Event.OnPlayerTriggersSector
[!] Trackmania.Event.OnPlayerThrowsObject
[!] Trackmania.Event.OnPlayerRequestActionChange
[!] Trackmania.Event.OnPlayerTriggersWaypoint,
[!] Trackmania.Event.OnVehicleArmorEmpty
[!] Trackmania.Event.OnVehicleCollision
[!] Trackmania.Event.OnVehicleVsVehicleCollision
```