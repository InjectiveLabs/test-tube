package testenv

import (
	"encoding/json"
	"strings"
	"time"

	// tendermint
	"cosmossdk.io/log"
	"cosmossdk.io/math"
	abci "github.com/cometbft/cometbft/abci/types"

	tmproto "github.com/cometbft/cometbft/proto/tendermint/types"
	tmtypes "github.com/cometbft/cometbft/types"

	dbm "github.com/cosmos/cosmos-db"
	"github.com/cosmos/cosmos-sdk/baseapp"
	"github.com/cosmos/cosmos-sdk/client/flags"
	"github.com/cosmos/cosmos-sdk/crypto/keys/secp256k1"
	"github.com/cosmos/cosmos-sdk/server"
	servertypes "github.com/cosmos/cosmos-sdk/server/types"
	simtestutil "github.com/cosmos/cosmos-sdk/testutil/sims"
	sdk "github.com/cosmos/cosmos-sdk/types"
	authtypes "github.com/cosmos/cosmos-sdk/x/auth/types"
	bankkeeper "github.com/cosmos/cosmos-sdk/x/bank/keeper"
	banktypes "github.com/cosmos/cosmos-sdk/x/bank/types"
	govtypes "github.com/cosmos/cosmos-sdk/x/gov/types"
	govv1types "github.com/cosmos/cosmos-sdk/x/gov/types/v1"

	// wasmd
	wasmtypes "github.com/CosmWasm/wasmd/x/wasm/types"

	// injective
	"github.com/InjectiveLabs/injective-core/injective-chain/app"
	injcodectypes "github.com/InjectiveLabs/injective-core/injective-chain/codec/types"
	exchangetypes "github.com/InjectiveLabs/injective-core/injective-chain/modules/exchange/types"
	tokenfactorytypes "github.com/InjectiveLabs/injective-core/injective-chain/modules/tokenfactory/types"
	wasmxtypes "github.com/InjectiveLabs/injective-core/injective-chain/modules/wasmx/types"
)

type TestEnv struct {
	App                *app.InjectiveApp
	Ctx                sdk.Context
	ParamTypesRegistry ParamTypeRegistry
	ValPrivs           []*secp256k1.PrivKey
	Validator          []byte
	NodeHome           string
}

type AppOptions map[string]interface{}

func (m AppOptions) Get(key string) interface{} {
	v, ok := m[key]
	if !ok {
		return nil
	}

	return v
}

func NewAppOptionsWithFlagHome(homePath string) servertypes.AppOptions {
	return AppOptions{
		flags.FlagHome:   homePath,
		server.FlagTrace: true,
	}
}

func NewInjectiveApp(nodeHome string) *app.InjectiveApp {
	db := dbm.NewMemDB()
	return app.NewInjectiveApp(
		log.NewNopLogger(),
		db,
		nil,
		true,
		NewAppOptionsWithFlagHome(nodeHome),
		baseapp.SetChainID("injective-777"),
	)
}

func InitChain(appInstance *app.InjectiveApp) (sdk.Context, secp256k1.PrivKey) {
	sdk.DefaultBondDenom = "inj"
	genesisState, valPriv := GenesisStateWithValSet(appInstance)

	encCfg := injcodectypes.MakeEncodingConfig()

	// Set up Wasm genesis state
	wasmGen := wasmtypes.GenesisState{
		Params: wasmtypes.Params{
			// Allow store code without gov
			CodeUploadAccess:             wasmtypes.AllowEverybody,
			InstantiateDefaultPermission: wasmtypes.AccessTypeEverybody,
		},
	}
	genesisState[wasmtypes.ModuleName] = encCfg.Codec.MustMarshalJSON(&wasmGen)

	// Set up governance genesis state
	govParams := govv1types.DefaultParams()
	votingPeriod := time.Second * 10 // 10 second
	govParams.VotingPeriod = &votingPeriod
	govGen := govv1types.GenesisState{
		StartingProposalId: govv1types.DefaultStartingProposalID,
		Deposits:           govv1types.Deposits{},
		Votes:              govv1types.Votes{},
		Proposals:          govv1types.Proposals{},
		Params:             &govParams,
	}

	genesisState[govtypes.ModuleName] = encCfg.Codec.MustMarshalJSON(&govGen)

	// Set up exchange genesis state
	exchangeParams := exchangetypes.DefaultParams()
	exchangeParams.IsInstantDerivativeMarketLaunchEnabled = true
	exchangeGen := exchangetypes.GenesisState{
		Params: exchangeParams,
	}
	genesisState[exchangetypes.ModuleName] = encCfg.Codec.MustMarshalJSON(&exchangeGen)

	// Set up wasmx genesis state
	wasmxGen := wasmxtypes.GenesisState{
		Params: wasmxtypes.Params{
			IsExecutionEnabled:    true,
			MaxBeginBlockTotalGas: 42000000,
			MaxContractGasLimit:   3500000,
			MinGasPrice:           1000,
		},
	}
	genesisState[wasmxtypes.ModuleName] = encCfg.Codec.MustMarshalJSON(&wasmxGen)

	stateBytes, err := json.MarshalIndent(genesisState, "", " ")

	requireNoErr(err)

	consensusParams := simtestutil.DefaultConsensusParams
	consensusParams.Block = &tmproto.BlockParams{
		MaxBytes: 22020096,
		MaxGas:   -1,
	}

	// replace sdk.DefaultDenom with "inj", a bit of a hack, needs improvement
	stateBytes = []byte(strings.Replace(string(stateBytes), "\"stake\"", "\"inj\"", -1))

	_, err = appInstance.InitChain(
		&abci.RequestInitChain{
			ChainId:         "injective-777",
			Validators:      []abci.ValidatorUpdate{},
			ConsensusParams: consensusParams,
			AppStateBytes:   stateBytes,
		},
	)
	requireNoErr(err)

	ctx := appInstance.NewUncachedContext(false, tmproto.Header{Height: 0, ChainID: "injective-777", Time: time.Now().UTC()})

	return ctx, valPriv
}

func GenesisStateWithValSet(appInstance *app.InjectiveApp) (app.GenesisState, secp256k1.PrivKey) {
	privVal := NewPV()
	pubKey, _ := privVal.GetPubKey()
	validator := tmtypes.NewValidator(pubKey, 1)
	valSet := tmtypes.NewValidatorSet([]*tmtypes.Validator{validator})

	// generate genesis account
	senderPrivKey := secp256k1.GenPrivKey()
	senderPrivKey.PubKey().Address()
	acc := authtypes.NewBaseAccountWithAddress(senderPrivKey.PubKey().Address().Bytes())

	//////////////////////
	// balances := []banktypes.Balance{}
	balance := banktypes.Balance{
		Address: acc.GetAddress().String(),
		Coins:   sdk.NewCoins(sdk.NewCoin(sdk.DefaultBondDenom, math.NewInt(100000000000000))),
	}
	genesisState := app.NewDefaultGenesisState()
	genAccs := []authtypes.GenesisAccount{acc}
	authGenesis := authtypes.NewGenesisState(authtypes.DefaultParams(), genAccs)
	genesisState[authtypes.ModuleName] = appInstance.AppCodec().MustMarshalJSON(authGenesis)

	genesisState, err := simtestutil.GenesisStateWithValSet(appInstance.AppCodec(), genesisState, valSet, []authtypes.GenesisAccount{acc}, balance)
	if err != nil {
		panic(err)
	}

	return genesisState, secp256k1.PrivKey{Key: privVal.PrivKey.Bytes()}
}

func (env *TestEnv) GetValidatorAddresses() []string {
	validators, err := env.App.StakingKeeper.GetAllValidators(env.Ctx)
	requireNoErr(err)

	var addresses []string
	for _, validator := range validators {
		addresses = append(addresses, validator.OperatorAddress)
	}

	return addresses
}

func (env *TestEnv) GetValidatorPrivateKey() []byte {
	return env.Validator
}

func (env *TestEnv) FundAccount(ctx sdk.Context, bankKeeper bankkeeper.Keeper, addr sdk.AccAddress, amounts sdk.Coins) error {
	if err := bankKeeper.MintCoins(ctx, tokenfactorytypes.ModuleName, amounts); err != nil {
		return err
	}

	return bankKeeper.SendCoinsFromModuleToAccount(ctx, tokenfactorytypes.ModuleName, addr, amounts)
}

func (env *TestEnv) SetupParamTypes() {
	pReg := env.ParamTypesRegistry

	pReg.RegisterParamSet(&tokenfactorytypes.Params{})
}

func requireNoErr(err error) {
	if err != nil {
		panic(err)
	}
}
