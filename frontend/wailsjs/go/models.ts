export namespace services {
	
	export class MonitoredFolder {
	    id: string;
	    path: string;
	    name: string;
	
	    static createFrom(source: any = {}) {
	        return new MonitoredFolder(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.id = source["id"];
	        this.path = source["path"];
	        this.name = source["name"];
	    }
	}
	export class RepoStatus {
	    path: string;
	    hasChanges?: boolean;
	    hasUnpushed?: boolean;
	    hasError: boolean;
	    errorMessage?: string;
	
	    static createFrom(source: any = {}) {
	        return new RepoStatus(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.path = source["path"];
	        this.hasChanges = source["hasChanges"];
	        this.hasUnpushed = source["hasUnpushed"];
	        this.hasError = source["hasError"];
	        this.errorMessage = source["errorMessage"];
	    }
	}
	export class ScanResult {
	    scannedPath: string;
	    totalRepositories: number;
	    withChanges: RepoStatus[];
	    withUnpushed: RepoStatus[];
	    clean: RepoStatus[];
	    errors: RepoStatus[];
	    executionTime: number;
	
	    static createFrom(source: any = {}) {
	        return new ScanResult(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.scannedPath = source["scannedPath"];
	        this.totalRepositories = source["totalRepositories"];
	        this.withChanges = this.convertValues(source["withChanges"], RepoStatus);
	        this.withUnpushed = this.convertValues(source["withUnpushed"], RepoStatus);
	        this.clean = this.convertValues(source["clean"], RepoStatus);
	        this.errors = this.convertValues(source["errors"], RepoStatus);
	        this.executionTime = source["executionTime"];
	    }
	
		convertValues(a: any, classs: any, asMap: boolean = false): any {
		    if (!a) {
		        return a;
		    }
		    if (a.slice && a.map) {
		        return (a as any[]).map(elem => this.convertValues(elem, classs));
		    } else if ("object" === typeof a) {
		        if (asMap) {
		            for (const key of Object.keys(a)) {
		                a[key] = new classs(a[key]);
		            }
		            return a;
		        }
		        return new classs(a);
		    }
		    return a;
		}
	}

}

